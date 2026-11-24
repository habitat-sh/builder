#!/bin/bash

service_config="{{pkg.svc_config_path}}/service.json"

# Read settings as data rather than interpolating credentials into shell code.
load_config() {
  if ! jq -e '
    def positive_integer: type == "number" and . == floor and . > 0;
    def port: positive_integer and . < 65536;
    (.key_id | type == "string" and length > 0) and
    (.secret_key | type == "string" and length > 0) and
    (.bucket_name | type == "string" and length > 0) and
    (.bind_address | type == "string" and length > 0) and
    (.bind_port | port) and
    (.internal.master_port | port and . < 55536) and
    (.internal.volume_port | port and . < 55536) and
    (.internal.filer_port | port and . < 55536) and
    (.internal.s3_grpc_port | port) and
    (.storage.volume_max | positive_integer) and
    (.storage.volume_size_limit_mb | positive_integer) and
    (.readiness_attempts | positive_integer) and
    (.readiness_delay_seconds | positive_integer)
  ' "$service_config" >/dev/null; then
    echo "Invalid SeaweedFS configuration: configure credentials, addresses, and positive numeric limits." >&2
    return 1
  fi

  AWS_ACCESS_KEY_ID=$(jq -r '.key_id' "$service_config")
  AWS_SECRET_ACCESS_KEY=$(jq -r '.secret_key' "$service_config")
  export AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY
  export AWS_DEFAULT_REGION=us-east-1 AWS_DEFAULT_OUTPUT=json AWS_MAX_ATTEMPTS=1
  export AWS_EC2_METADATA_DISABLED=true AWS_PAGER=""
  export AWS_CONFIG_FILE=/dev/null AWS_SHARED_CREDENTIALS_FILE=/dev/null
  bucket_name=$(jq -r '.bucket_name' "$service_config")
  bind_address=$(jq -r '.bind_address' "$service_config")
  bind_port=$(jq -r '.bind_port' "$service_config")
  local client_address="$bind_address"
  case "$client_address" in
    0.0.0.0) client_address=127.0.0.1 ;;
    ::) client_address='[::1]' ;;
    *:*) client_address="[$client_address]" ;;
  esac
  s3_endpoint="http://${client_address}:${bind_port}"
}

s3api() {
  aws --endpoint-url "$s3_endpoint" --cli-connect-timeout 2 --cli-read-timeout 5 s3api "$@"
}

ensure_bucket() {
  local attempts delay buckets
  attempts=$(jq -r '.readiness_attempts' "$service_config")
  delay=$(jq -r '.readiness_delay_seconds' "$service_config")
  for ((attempt = 1; attempt <= attempts; attempt++)); do
    if buckets=$(s3api list-buckets 2>/dev/null); then
      if jq -e --arg bucket "$bucket_name" '.Buckets | any(.Name == $bucket)' <<<"$buckets" >/dev/null; then
        echo "SeaweedFS artifact bucket is ready."
        return 0
      fi
      # Also retry creation: the S3 listener can be ready before volumes are writable.
      if s3api create-bucket --bucket "$bucket_name" >/dev/null; then
        echo "Created SeaweedFS artifact bucket."
        return 0
      fi
    fi
    if ((attempt < attempts)); then sleep "$delay"; fi
  done
  echo "SeaweedFS bucket initialization failed; check credentials, service logs, and storage availability." >&2
  return 1
}
