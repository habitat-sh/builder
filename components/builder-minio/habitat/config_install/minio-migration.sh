#!/bin/bash

function download_bucket_objects() {
  echo "DOWNLOADING objects from the MinIO that we are migrating from"
  if ! aws --endpoint-url "$MINIO_ENDPOINT" s3 sync "$s3_url" "$WAYPOINT"; then
    echo "ERROR: download of objects FAILED"
    exit 8
  fi
  return 0
}

function _ensure_bucket_exists() {
  if ! aws --endpoint-url "$MINIO_ENDPOINT" s3 ls "{{cfg.bucket_name}}" &>/dev/null; then
    aws --endpoint-url "$MINIO_ENDPOINT" s3 mb "$s3_url"
  fi
}

function upload_bucket_objects() {
  _ensure_bucket_exists
  echo "UPLOADING objects to the MinIO that we are migrating to"
  if ! aws --endpoint-url "$MINIO_ENDPOINT" s3 sync "$WAYPOINT" "$s3_url"; then
    echo "ERROR: upload of objects FAILED"
    exit 9
  fi
  return 0
}

function is_migration_from_removed_fs_backend_needed() {
  echo "CHECKING if migration from removed fs backend is needed"
  if [[ -f /hab/svc/builder-minio/data/.minio.sys/format.json ]]; then
    format_value=$(jq -r '.format' /hab/svc/builder-minio/data/.minio.sys/format.json)
    if [[ "${format_value}" == 'fs' ]]; then
      return 0
    fi
  fi
  return 1
}

function minio_health_live_check() {
  for ((n = 0; n < 20; n++)); do
    curl_http_code_args=(-fs -o /dev/null -w "%{http_code}" --retry 4 --retry-delay 1)
    code=$(curl "${curl_http_code_args[@]}" "$MINIO_ENDPOINT/minio/health/live")
    if [[ $code == 200 ]]; then
      return 0
    else
      sleep .5
    fi
  done
  return 1
}

function minio_stop() {
  for ((n = 0; n < 20; n++)); do
    if pgrep minio &>/dev/null; then
      pkill minio &>/dev/null
      sleep 1
    else
      return 0
    fi
  done
}

function config_environment_for_migration() {
  if [[ ! $1 =~ ^[0-9]+$ ]]; then
    echo "ERROR: Invalid timestamp"
  fi
  if [ -f "{{pkg.svc_files_path}}/private.key" ]; then
    MINIO_ENDPOINT="https://localhost:{{cfg.bind_port}}"
  else
    MINIO_ENDPOINT="http://localhost:{{cfg.bind_port}}"
  fi
  export MINIO_ENDPOINT
  export AWS_ACCESS_KEY_ID="{{cfg.env.MINIO_ACCESS_KEY}}"
  export AWS_SECRET_ACCESS_KEY="{{cfg.env.MINIO_SECRET_KEY}}"
  export s3_url="s3://{{cfg.bucket_name}}"
  WAYPOINT=$(mktemp -d -t minio-waypoint-"$1"-XXXXXXXXXX)
  export WAYPOINT
}

function _enumerate_bucket_objects() {
  aws --endpoint-url "$MINIO_ENDPOINT" s3 ls "$s3_url" --recursive --summarize >"$1"
}

function summarize_old_minio_bucket_objects() {
  if [[ ! $1 =~ ^[0-9]+$ ]]; then
    echo "Invalid timestamp"
  fi
  local tempfile
  tempfile=$(mktemp -t "minio-old-contents-summary-$1-XXXXXXXXXX")
  _enumerate_bucket_objects "$tempfile"
  echo "$tempfile"
}

function summarize_new_minio_bucket_objects() {
  if [[ ! $1 =~ ^[0-9]+$ ]]; then
    echo "ERROR: Invalid timestamp"
  fi
  local tempfile
  tempfile=$(mktemp -t "minio-new-contents-summary-$1-XXXXXXXXXX")
  _enumerate_bucket_objects "$tempfile"
  echo "$tempfile"
}
