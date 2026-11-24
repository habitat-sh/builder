#!/bin/bash
# Run against a disposable service using AWS credentials from the environment.
set -euo pipefail

endpoint=${1:?Usage: smoke.sh ENDPOINT BUCKET}
bucket=${2:?Usage: smoke.sh ENDPOINT BUCKET}
export AWS_DEFAULT_REGION=us-east-1 AWS_EC2_METADATA_DISABLED=true AWS_PAGER=""
workdir=$(mktemp -d)
key="seaweedfs-smoke-$(basename "$workdir")"

s3api() { aws --endpoint-url "$endpoint" s3api "$@"; }
cleanup() {
  s3api delete-object --bucket "$bucket" --key "$key" >/dev/null 2>&1 || true
  rm -rf "$workdir"
}
trap cleanup EXIT

s3api head-bucket --bucket "$bucket"
printf 'Builder SeaweedFS package smoke test\n' >"$workdir/source"
s3api put-object --bucket "$bucket" --key "$key" --body "$workdir/source" >/dev/null
s3api get-object --bucket "$bucket" --key "$key" "$workdir/download" >/dev/null
cmp "$workdir/source" "$workdir/download"
s3api delete-object --bucket "$bucket" --key "$key" >/dev/null
if s3api head-object --bucket "$bucket" --key "$key" >"$workdir/head-output" 2>&1; then
  echo "Deleted object is still accessible" >&2
  exit 1
fi
grep -q '(404)' "$workdir/head-output"
echo "SeaweedFS S3 smoke test passed."
