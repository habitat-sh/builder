#!/bin/bash
# Package checks against the functional job's already-running SeaweedFS service.
# Supervisor and service setup/teardown belong to the shared Studio setup.
set -euo pipefail

readonly endpoint=${1:?Usage: integration.sh ENDPOINT BUCKET SUPERVISOR_LOG}
readonly bucket=${2:?Usage: integration.sh ENDPOINT BUCKET SUPERVISOR_LOG}
readonly supervisor_log=${3:?Usage: integration.sh ENDPOINT BUCKET SUPERVISOR_LOG}
readonly service=habitat/builder-seaweedfs
readonly package=$service
: "${AWS_ACCESS_KEY_ID:?Configure the service access key}"
: "${AWS_SECRET_ACCESS_KEY:?Configure the service secret key}"
export AWS_DEFAULT_REGION=us-east-1 AWS_MAX_ATTEMPTS=1
export AWS_EC2_METADATA_DISABLED=true AWS_PAGER=""
export AWS_CONFIG_FILE=/dev/null AWS_SHARED_CREDENTIALS_FILE=/dev/null
workdir=$(mktemp -d)
readonly workdir
trap 'rm -rf "$workdir"' EXIT

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
  echo "Timed out waiting for $message; see the Supervisor log." >&2
  return 1
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

wait_for 'automatic artifact bucket creation' bucket_ready

echo "--- Checking credential-file permissions and rendered JSON"
for config in service.json s3.json; do
  test "$(stat -c %a "/hab/svc/builder-seaweedfs/config/$config")" = 600
  hab pkg exec "$package" jq empty "/hab/svc/builder-seaweedfs/config/$config"
done

echo "--- Testing S3 upload, download, and deletion"
hab pkg exec "$package" bash "$(dirname "${BASH_SOURCE[0]}")/smoke.sh" \
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
  grep -q 'SeaweedFS artifact bucket is ready.' "$supervisor_log"
s3api delete-object --bucket "$bucket" --key persistence-test 2>&1
echo "SeaweedFS package tests passed."
