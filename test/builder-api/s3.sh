#!/bin/bash
# Runs after Mocha and before fixture cleanup, in either S3 functional Studio.
set -euo pipefail
cd /src

readonly api=http://localhost:9636
readonly endpoint=${BUILDER_TEST_S3_ENDPOINT:-http://localhost:9000}
readonly bucket=${BUILDER_TEST_S3_BUCKET:-habitat-builder-artifact-store.default}
export AWS_ACCESS_KEY_ID=${BUILDER_TEST_S3_KEY_ID:-depot}
export AWS_SECRET_ACCESS_KEY=${BUILDER_TEST_S3_SECRET_KEY:-password}
export AWS_DEFAULT_REGION=us-east-1 AWS_EC2_METADATA_DISABLED=true AWS_PAGER=""
workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

s3api() {
  hab pkg exec core/aws-cli aws --endpoint-url "$endpoint" \
    --cli-connect-timeout 2 --cli-read-timeout 30 s3api "$@"
}

echo "--- Generating a signed multipart artifact"
hab origin key generate s3test
HAB_ORIGIN=s3test build /src/test/builder-api/fixtures/s3-large \
  2>&1 | tee logs/multipart-fixture.build.log
# shellcheck disable=SC1091
source /src/results/last_build.env
readonly artifact="/src/results/$pkg_artifact"
readonly ident=$pkg_ident
readonly object_key="$ident/x86_64/linux/$pkg_artifact"
# Check actual HART size, not just the uncompressed payload's size.
test "$(stat -c %s "$artifact")" -gt $((20 * 1024 * 1024))

hab origin create s3test --url "$api" --auth bobo
hab origin key upload s3test --url "$api" --auth bobo
echo "--- Uploading the large artifact through Builder"
hab pkg upload "$artifact" --url "$api" --auth bobo
s3api head-object --bucket "$bucket" --key "$object_key"
# This log is emitted by Builder's multipart implementation, not the AWS CLI.
grep -F "S3Handler::multipart_upload started for s3_key: $object_key" logs/sup.log
grep -E 'Multipart upload completed .*s3-large.* with 3 parts' logs/sup.log

echo "--- Downloading through Builder and verifying every byte"
curl --fail --silent --show-error --max-time 120 \
  -H 'Authorization: Bearer bobo' \
  "$api/v1/depot/pkgs/$ident/download?target=x86_64-linux" \
  --output "$workdir/download.hart"
cmp "$artifact" "$workdir/download.hart"

echo "--- Deleting through Builder and checking the backing object"
hab pkg delete "$ident" --url "$api" --auth bobo
if s3api head-object --bucket "$bucket" --key "$object_key" >"$workdir/deleted" 2>&1; then
  echo "Builder deleted the record but left its S3 object behind." >&2
  exit 1
fi
cat "$workdir/deleted"
grep -q '(404)' "$workdir/deleted"
s3api list-multipart-uploads --bucket "$bucket" --prefix "$object_key" \
  >"$workdir/multipart.json"
hab pkg exec core/jq-static jq -e '(.Uploads // []) | length == 0' "$workdir/multipart.json"
echo "Builder S3 multipart integration test passed."
