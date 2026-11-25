#!/bin/bash
# This test requires an isolated CI Studio; it never starts Builder or MinIO.
set -euo pipefail
cd /src

readonly logs=/src/logs/seaweedfs
readonly service=habitat/builder-seaweedfs
readonly endpoint=http://127.0.0.1:19090
readonly bucket=builder-seaweedfs-ci
mkdir -p "$logs"
# Keep assertion files out of the artifacts and clean them up on every exit path.
workdir=$(mktemp -d)
readonly workdir
trap 'rm -rf "$workdir"' EXIT

if hab svc status >/dev/null 2>&1; then
  echo "SeaweedFS package tests require a Studio without a running Supervisor." >&2
  exit 1
fi

# Only SeaweedFS needs unstable. Other dependencies retain the pipeline defaults.
hab pkg install core/seaweedfs --channel unstable
echo "--- Building builder-seaweedfs"
HAB_PREFER_LOCAL_CHEF_DEPS=true build /src/components/builder-seaweedfs \
  2>&1 | tee "$logs/build.log"
# shellcheck disable=SC1091
source /src/results/last_build.env
readonly package="$pkg_ident"

# Stream tests to the console and test.log; keep Supervisor output separate.
exec 3>&1
exec > >(tee "$logs/test.log") 2>&1

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

supervisor_ready() {
  # Connection failures are expected while the Supervisor is starting.
  hab svc status >"$workdir/supervisor-readiness.log" 2>&1
}

bucket_ready() {
  s3api head-bucket --bucket "$bucket" 2>&1
}

service_stopped() {
  # Closing the S3 listener precedes SeaweedFS process termination. Starting
  # again before this PID exits can reattach to the process being stopped.
  if kill -0 "$old_service_pid" 2>/dev/null; then return 1; fi
  if s3api head-bucket --bucket "$bucket" 2>&1 | tee "$workdir/stopped.log"; then
    return 1
  fi
  grep -q 'Could not connect to the endpoint URL' "$workdir/stopped.log"
}

persistent_object_ready() {
  # HeadBucket can succeed before master election and volume registration finish.
  # Retry transport/read failures; compare the contents separately after success.
  s3api get-object --bucket "$bucket" --key persistence-test \
    "$workdir/persistence-download.txt" 2>&1
}

cleanup() {
  local result=$?
  trap - EXIT
  hab svc status 2>&1 || true
  hab sup term 2>&1 || true
  wait "$supervisor_pid" || true
  rm -rf "$workdir"
  exit "$result"
}

echo "--- Starting Supervisor and SeaweedFS"
# Process substitution streams output while keeping $! as the Supervisor PID.
hab sup run --no-color > >(tee "$logs/sup.log" >&3) 2>&1 &
supervisor_pid=$!
trap cleanup EXIT
if ! wait_for Supervisor supervisor_ready; then
  cat "$workdir/supervisor-readiness.log" >&2
  tail -n 60 "$logs/sup.log" >&2
  exit 1
fi
hab svc load "$package"
wait_for 'automatic artifact bucket creation' bucket_ready

echo "--- Checking credential-file permissions and rendered JSON"
for config in service.json s3.json; do
  test "$(stat -c %a "/hab/svc/builder-seaweedfs/config/$config")" = 600
  hab pkg exec "$package" jq empty "/hab/svc/builder-seaweedfs/config/$config"
done

echo "--- Testing S3 upload, download, and deletion"
hab pkg exec "$package" bash /src/components/builder-seaweedfs/tests/smoke.sh \
  "$endpoint" "$bucket" 2>&1

echo "--- Checking that incorrect credentials are rejected"
if AWS_SECRET_ACCESS_KEY=incorrect s3api head-bucket --bucket "$bucket" \
    2>&1 | tee "$workdir/rejected-credentials.log"; then
  echo "SeaweedFS accepted incorrect credentials." >&2
  exit 1
fi
# An unavailable endpoint must not count as successful authentication rejection.
grep -Eq 'SignatureDoesNotMatch|InvalidAccessKeyId|AccessDenied|\(403\)' \
  "$workdir/rejected-credentials.log"

echo "--- Testing persistence across a completed stop/start"
printf 'SeaweedFS package persistence test\n' >"$workdir/persistence-source.txt"
s3api put-object --bucket "$bucket" --key persistence-test \
  --body "$workdir/persistence-source.txt" 2>&1
old_service_pid=$(cat /hab/svc/builder-seaweedfs/PID)
hab svc stop "$service"
wait_for 'SeaweedFS process termination and closed S3 listener' service_stopped
hab svc start "$service"
wait_for 'SeaweedFS to restart' bucket_ready
wait_for 'persisted object to become readable after restart' persistent_object_ready
cmp "$workdir/persistence-source.txt" "$workdir/persistence-download.txt"
# The existing bucket must be reused, rather than recreated, on restart.
wait_for 'idempotent bucket initialization' \
  grep -q 'SeaweedFS artifact bucket is ready.' "$logs/sup.log"
s3api delete-object --bucket "$bucket" --key persistence-test 2>&1
echo "SeaweedFS package tests passed."
