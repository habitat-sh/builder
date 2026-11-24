#!/bin/bash
# This test requires an isolated CI Studio; it never starts Builder or MinIO.
set -euo pipefail
cd /src

readonly logs=/src/logs/seaweedfs
readonly service=habitat/builder-seaweedfs
readonly endpoint=http://127.0.0.1:19090
readonly bucket=builder-seaweedfs-ci
mkdir -p "$logs"

if hab svc status >/dev/null 2>&1; then
  echo "SeaweedFS package tests require a Studio without a running Supervisor." >&2
  exit 1
fi

# Only SeaweedFS needs unstable. Other dependencies retain the pipeline defaults.
hab pkg install core/seaweedfs --channel unstable
echo "--- Building builder-seaweedfs"
HAB_PREFER_LOCAL_CHEF_DEPS=true build /src/components/builder-seaweedfs \
  >"$logs/build.log" 2>&1
# shellcheck disable=SC1091
source /src/results/last_build.env
readonly package="$pkg_ident"

mkdir -p /hab/user/builder-seaweedfs/config
# Fixed, non-production credentials with characters that must survive rendering.
cat >/hab/user/builder-seaweedfs/config/user.toml <<'TOML'
bind_address = "127.0.0.1"
bind_port = 19090
bucket_name = "builder-seaweedfs-ci"
key_id = "builder-seaweedfs-ci"
secret_key = 'ci-"quote-$dollar-`backtick`-\backslash'
[storage]
volume_max = 8
volume_size_limit_mb = 64
TOML
chmod 0600 /hab/user/builder-seaweedfs/config/user.toml

export AWS_ACCESS_KEY_ID=builder-seaweedfs-ci
# Intentionally literal: exercise JSON escaping without evaluating shell syntax.
# shellcheck disable=SC2016
export AWS_SECRET_ACCESS_KEY='ci-"quote-$dollar-`backtick`-\backslash'
export AWS_DEFAULT_REGION=us-east-1 AWS_MAX_ATTEMPTS=1
export AWS_EC2_METADATA_DISABLED=true AWS_PAGER=""
export AWS_CONFIG_FILE=/dev/null AWS_SHARED_CREDENTIALS_FILE=/dev/null

s3api() {
  hab pkg exec "$package" aws --endpoint-url "$endpoint" \
    --cli-connect-timeout 2 --cli-read-timeout 5 s3api "$@"
}

wait_for() {
  local message=$1
  shift
  local attempt
  for ((attempt = 1; attempt <= 60; attempt++)); do
    if "$@"; then return 0; fi
    sleep 1
  done
  echo "Timed out waiting for $message; see logs/seaweedfs." >&2
  return 1
}

bucket_ready() {
  s3api head-bucket --bucket "$bucket" >"$logs/readiness.log" 2>&1
}

service_stopped() {
  # Closing the S3 listener precedes SeaweedFS process termination. Starting
  # again before this PID exits can reattach to the process being stopped.
  if kill -0 "$old_service_pid" 2>/dev/null; then return 1; fi
  if s3api head-bucket --bucket "$bucket" >"$logs/stopped.log" 2>&1; then
    return 1
  fi
  grep -q 'Could not connect to the endpoint URL' "$logs/stopped.log"
}

persistent_object_ready() {
  # HeadBucket can succeed before master election and volume registration finish.
  # Retry transport/read failures; compare the contents separately after success.
  s3api get-object --bucket "$bucket" --key persistence-test \
    "$logs/persistence-download.txt" >"$logs/get-object.log" 2>&1
}

cleanup() {
  local result=$?
  trap - EXIT
  hab svc status >"$logs/status.log" 2>&1 || true
  hab sup term >"$logs/shutdown.log" 2>&1 || true
  wait "$supervisor_pid" || true
  exit "$result"
}

echo "--- Starting Supervisor and SeaweedFS"
hab sup run --no-color >"$logs/supervisor.log" 2>&1 &
supervisor_pid=$!
trap cleanup EXIT
wait_for Supervisor hab svc status
hab svc load "$package"
wait_for 'automatic artifact bucket creation' bucket_ready

echo "--- Checking credential-file permissions and rendered JSON"
for config in service.json s3.json; do
  test "$(stat -c %a "/hab/svc/builder-seaweedfs/config/$config")" = 600
  hab pkg exec "$package" jq empty "/hab/svc/builder-seaweedfs/config/$config"
done

echo "--- Testing S3 upload, download, and deletion"
hab pkg exec "$package" bash /src/components/builder-seaweedfs/tests/smoke.sh \
  "$endpoint" "$bucket" >"$logs/smoke.log" 2>&1

echo "--- Checking that incorrect credentials are rejected"
if AWS_SECRET_ACCESS_KEY=incorrect s3api head-bucket --bucket "$bucket" \
    >"$logs/rejected-credentials.log" 2>&1; then
  echo "SeaweedFS accepted incorrect credentials." >&2
  exit 1
fi
# An unavailable endpoint must not count as successful authentication rejection.
grep -Eq 'SignatureDoesNotMatch|InvalidAccessKeyId|AccessDenied|\(403\)' \
  "$logs/rejected-credentials.log"

echo "--- Testing persistence across a completed stop/start"
printf 'SeaweedFS package persistence test\n' >"$logs/persistence-source.txt"
s3api put-object --bucket "$bucket" --key persistence-test \
  --body "$logs/persistence-source.txt" >"$logs/put-object.log"
old_service_pid=$(cat /hab/svc/builder-seaweedfs/PID)
hab svc stop "$service" --shutdown-timeout 30
wait_for 'SeaweedFS process termination and closed S3 listener' service_stopped
hab svc start "$service"
wait_for 'SeaweedFS to restart' bucket_ready
wait_for 'persisted object to become readable after restart' persistent_object_ready
cmp "$logs/persistence-source.txt" "$logs/persistence-download.txt"
# The existing bucket must be reused, rather than recreated, on restart.
wait_for 'idempotent bucket initialization' \
  grep -q 'SeaweedFS artifact bucket is ready.' "$logs/supervisor.log"
s3api delete-object --bucket "$bucket" --key persistence-test >"$logs/delete-object.log"
echo "SeaweedFS package tests passed."
