#!/bin/bash

# JAH: Consider this function more closely and maybe eliminated it
function sudo () {
  [[ $EUID = 0 ]] || set -- command sudo -E "$@"
  "$@"
}

function _hab_exec () {
  pkg=$1; shift
  hab pkg exec "$pkg" -- "$@"
}

function _aws () {
  _hab_exec 'core/aws-cli' aws --endpoint-url "{{cfg.env.MINIO_ENDPOINT}}" "$@"
}

function _curl () {
  _hab_exec 'core/curl' curl "$@"
}

function _jq () {
  _hab_exec 'core/jq-static' jq "$@"
}


function _hab_pkg_install () {
  pkg=$1; shift
  echo "$pkg not installed, installing"
  if ! sudo hab pkg install "$pkg" -- "$@"
  then    
    echo "ERROR: install of $pkg FAILED"
    exit 4
  fi
  return 0
}

function _are_dependencies_installed () {
  echo "CHECKING dependencies"
  declare -a deps
  deps=( 'core/aws-cli' 'core/curl' 'core/jq-static' )
  for d in "${deps[@]}"; do
    if hab pkg env "$d" &> /dev/null; then
      if ! _hab_pkg_install "$d"; then
        return 5
      fi
    fi
  done
}

function _minio_check {
  echo "CHECKING MinIO"
  local output
  output=$( _aws s3 ls )
  if [[ ! $output =~ $MINIO_BUCKET ]]; then
    echo "ERROR: Invalid MinIO credentials"
    return 6
  fi
  return 0
}

function preflight_checks () {
  if ! _are_dependencies_installed
  then
    echo "ERROR: one or more preflight checks FAILED"
    exit 7
  fi
}

function download_bucket_objects () {
  echo "DOWNLOADING objects from the MinIO that we are migrating from"
  if ! _aws s3 sync "$s3_url" "$WAYPOINT"; then
    echo "ERROR: download of objects FAILED"
    exit 8
  fi
  return 0
}

function _ensure_bucket_exists () {
  if ! _aws s3 ls "$MINIO_BUCKET" &> /dev/null; then
    _aws s3 mb "$s3_url"
  fi
}

function upload_bucket_objects () {
  _ensure_bucket_exists
  echo "UPLOADING objects to the MinIO that we are migrating to"
  if ! _aws s3 sync "$WAYPOINT" "$s3_url"; then
    echo "ERROR: upload of objects FAILED"
    exit 9
  fi
  return 0
}

function is_migration_from_removed_fs_backend_needed() {
  echo "CHECKING if migration from removed fs backend is needed"
  if [[ -f /hab/svc/builder-minio/data/.minio.sys/format.json ]]
  then
    format_value=$(_jq -r '.format' /hab/svc/builder-minio/data/.minio.sys/format.json)
    if [[ "${format_value}" == 'fs' ]]
    then
      return 0
    fi
  fi
  return 1
}

function minio_health_live_check () {
  for (( n=0; n<20; n++ ))
  do 
    curl_http_code_args=( -fs -o /dev/null -w "%{http_code}" --retry 4 --retry-delay 1 )
    code=$(_curl "${curl_http_code_args[@]}" "$MINIO_ENDPOINT/minio/health/live")
    if [[ $code == 200 ]]
    then 
      return 0
    else 
      sleep .5
    fi
  done
  return 1
}

function minio_stop () {
  for (( n=0; n<20; n++ ))
  do 
    if pgrep minio > /dev/null
    then
      sudo pkill minio > /dev/null
      sleep 1
    else
      return 0
    fi
  done
}

function config_environment_for_migration () {
  if [[ ! $1 =~ ^[0-9]+$ ]]; then
    echo "ERROR: Invalid timestamp"
  fi
  export MINIO_ENDPOINT="{{cfg.env.MINIO_ENDPOINT}}"
  export AWS_ACCESS_KEY_ID="{{cfg.env.MINIO_ACCESS_KEY}}"
  export AWS_SECRET_ACCESS_KEY="{{cfg.env.MINIO_SECRET_KEY}}"
  export s3_url="s3://{{cfg.env.MINIO_BUCKET}}"
  export MINIO_BUCKET="{{cfg.env.MINIO_BUCKET}}"
  WAYPOINT=$( mktemp -d -t minio-waypoint-"$1"-XXXXXXXXXX )
  export WAYPOINT
}

function _enumerate_bucket_objects () {
  _aws s3 ls "$s3_url" --recursive --summarize > "$1"
}

function summarize_old_minio_bucket_objects () {
  if [[ ! $1 =~ ^[0-9]+$ ]]; then
    echo "Invalid timestamp"
  fi
  local tempfile
  tempfile=$(mktemp -t "minio-old-contents-summary-$1-XXXXXXXXXX")
  _enumerate_bucket_objects "$tempfile"
  echo "$tempfile"
}

function summarize_new_minio_bucket_objects () {
  if [[ ! $1 =~ ^[0-9]+$ ]]; then
    echo "ERROR: Invalid timestamp"
  fi
  local tempfile
  tempfile=$(mktemp -t "minio-new-contents-summary-$1-XXXXXXXXXX")
  _enumerate_bucket_objects "$tempfile"
  echo "$tempfile"
}
