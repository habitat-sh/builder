#!/bin/bash

set -eou pipefail
umask 0022 

sudo () {
  [[ $EUID = 0 ]] || set -- command sudo -E "$@"
  "$@"
}
export -f sudo

user_toml_warn() {
  if [ -f "/hab/svc/$1/user.toml" ]; then
      mv "/hab/svc/$1/user.toml" "/hab/svc/$1/user.toml.bak"
      echo "WARNING: Previous user.toml exists in deprecated location. All user.toml" 
      echo "files should be deposited into the path /hab/user/$1/config/user.toml."
      echo "Deprecated user.toml has been renamed user.toml.bak."
  fi
}
export -f user_toml_warn

init_datastore() {
  user_toml_warn builder-datastore
  mkdir -p /hab/user/builder-datastore/config
  cat <<EOT > /hab/user/builder-datastore/config/user.toml
max_locks_per_transaction = 128
dynamic_shared_memory_type = 'none'

[superuser]
name = 'hab'
password = 'hab'
EOT
}
export -f init_datastore

configure() {
  export PGPASSWORD PGUSER
    if [ "${PG_EXT_ENABLED:-false}" = "true" ]; then
      PGUSER=${PG_USER:-hab}
      PGPASSWORD=${PG_PASSWORD:-hab}
    else
      PGUSER="hab"
      PGPASSWORD=""
    fi

  export ANALYTICS_ENABLED=${ANALYTICS_ENABLED:="false"}
  export ANALYTICS_COMPANY_ID
  export ANALYTICS_COMPANY_NAME
  export ANALYTICS_WRITE_KEY

  if [ $ANALYTICS_ENABLED = "true" ]; then
    ANALYTICS_WRITE_KEY=${ANALYTICS_WRITE_KEY:="NAwVPW04CeESMW3vtyqjJZmVMNBSQ1K1"}
    ANALYTICS_COMPANY_ID=${ANALYTICS_COMPANY_ID:="builder-on-prem"}
  else
    ANALYTICS_WRITE_KEY=""
    ANALYTICS_COMPANY_ID=""
    ANALYTICS_COMPANY_NAME=""
  fi

  export LOAD_BALANCED="false"
  if [ "${HAB_BLDR_PEER_ARG:-}" != "" ]; then
    LOAD_BALANCED="true"
  fi

  # don't write out the builder-minio user.toml if using S3 or Artifactory directly
  if [ "${S3_ENABLED:-false}" = "false" ] && [ "${ARTIFACTORY_ENABLED:-false}" = "false" ]; then
    if [ "${FRONTEND_INSTALL:-0}" != 1 ]; then
      user_toml_warn builder-minio
      mkdir -p /hab/user/builder-minio/config
      cat <<EOT > /hab/user/builder-minio/config/user.toml
key_id = "$MINIO_ACCESS_KEY"
secret_key = "$MINIO_SECRET_KEY"
bucket_name = "$MINIO_BUCKET"
EOT
    fi
  fi

  mkdir -p /hab/user/builder-api/config
  export S3_BACKEND="minio"
  if [ "${S3_ENABLED:-false}" = "true" ]; then
    S3_BACKEND="aws"
    MINIO_ENDPOINT=$S3_REGION
    MINIO_ACCESS_KEY=$S3_ACCESS_KEY
    MINIO_SECRET_KEY=$S3_SECRET_KEY
    MINIO_BUCKET=$S3_BUCKET
  fi
  if [ "${ARTIFACTORY_ENABLED:-false}" = "true" ]; then
    FEATURES_ENABLED="jobsrv ARTIFACTORY"
  else
    FEATURES_ENABLED="jobsrv"
    ARTIFACTORY_API_URL="http://localhost:8081"
    ARTIFACTORY_API_KEY="none"
    ARTIFACTORY_REPO="habitat-builder-artifact-store"
  fi
  PG_HOST=${POSTGRES_HOST:-localhost}
  PG_PORT=${POSTGRES_PORT:-5432}
  user_toml_warn builder-api
  cat <<EOT > /hab/user/builder-api/config/user.toml
log_level="debug"
jobsrv_enabled = true

[http]
handler_count = 10

[github]
app_id = 8053
webhook_secret = ""
[api]
features_enabled = "$FEATURES_ENABLED"
targets = ["x86_64-linux", "x86_64-linux-kernel2", "x86_64-windows"]

[depot]
jobsrv_enabled = true

[oauth]
provider = "$OAUTH_PROVIDER"
userinfo_url = "$OAUTH_USERINFO_URL"
token_url = "$OAUTH_TOKEN_URL"
redirect_url = "$OAUTH_REDIRECT_URL"
client_id = "$OAUTH_CLIENT_ID"
client_secret = "$OAUTH_CLIENT_SECRET"

[s3]
backend = "$S3_BACKEND"
key_id = "$MINIO_ACCESS_KEY"
secret_key = "$MINIO_SECRET_KEY"
endpoint = "$MINIO_ENDPOINT"
bucket_name = "$MINIO_BUCKET"

[artifactory]
api_url = "$ARTIFACTORY_API_URL"
api_key = "$ARTIFACTORY_API_KEY"
repo = "$ARTIFACTORY_REPO"

[memcache]
ttl = 15

[datastore]
user = "$PGUSER"
password = "$PGPASSWORD"
connection_timeout_sec = 5
host = "$PG_HOST"
port = $PG_PORT
ssl_mode = "prefer"
EOT
user_toml_warn builder-api-proxy
mkdir -p /hab/user/builder-api-proxy/config
cat <<EOT > /hab/user/builder-api-proxy/config/user.toml
log_level="info"
enable_builder = true
app_url = "${APP_URL}"
load_balanced = ${LOAD_BALANCED}

[oauth]
provider = "$OAUTH_PROVIDER"
client_id = "$OAUTH_CLIENT_ID"
authorize_url = "$OAUTH_AUTHORIZE_URL"
redirect_url = "$OAUTH_REDIRECT_URL"
signup_url = "$OAUTH_SIGNUP_URL"

[nginx]
max_body_size = "2048m"
proxy_send_timeout = 180
proxy_read_timeout = 180
enable_gzip = true
enable_caching = true
limit_req_zone_unknown = "\$limit_unknown zone=unknown:10m rate=30r/s"
limit_req_unknown      = "burst=90 nodelay"
limit_req_zone_known   = "\$http_x_forwarded_for zone=known:10m rate=30r/s"
limit_req_known        = "burst=90 nodelay"

[http]
keepalive_timeout = "180s"

[server]
listen_tls = $APP_SSL_ENABLED

[analytics]
enabled = $ANALYTICS_ENABLED
company_id = "$ANALYTICS_COMPANY_ID"
company_name = "$ANALYTICS_COMPANY_NAME"
write_key = "$ANALYTICS_WRITE_KEY"
EOT
}
export -f configure

function psql() {
  PGPASSWORD=$(cat /hab/svc/builder-datastore/config/pwfile) hab pkg exec core/postgresql psql -U hab -h 127.0.0.1 -p 5432 "$@"
}
export -f psql

clean_test_artifacts() {
  echo "Performing DB cleanup"
  local sql origins
  origins=( neurosis )

  # clean origins
  local origins origin_tables
  origin_tables=( origin_integrations origin_project_integrations origin_secrets origin_private_encryption_keys origin_public_encryption_keys origin_members origin_channels origin_invitations origin_packages origin_projects origin_public_keys origin_secret_keys audit_package audit_package_group origin_package_settings )
  sql=

  for origin in "${origins[@]}"; do
    sql+="DELETE FROM origin_channel_packages WHERE channel_id IN (SELECT id FROM origin_channels WHERE origin='$origin');"

    for table in "${origin_tables[@]}"; do
      sql+="DELETE FROM $table WHERE origin='$origin';"
    done

    sql+="DELETE FROM origins WHERE name='$origin';"
  done

  psql builder -q -c "$sql"

  # clean users
  local users account_tables
  users=( bobo mystique lkennedy )
  sql=

  for user in "${users[@]}"; do
    sql+="DELETE FROM accounts WHERE name='$user';"
  done

  psql builder -q -c "$sql"

  # clean jobs
  sql=

  for origin in "${origins[@]}"; do
    sql+="DELETE FROM busy_workers WHERE job_id IN (SELECT id FROM jobs WHERE project_name LIKE '$origin%');"
    sql+="DELETE FROM group_projects WHERE project_name LIKE '$origin%';"
    sql+="DELETE FROM groups WHERE project_name LIKE '$origin%';"
    sql+="DELETE FROM jobs WHERE project_name LIKE '$origin%';"
  done

  psql builder -q -c "$sql"
}
export -f clean_test_artifacts

start_api() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-api" --bind memcached:builder-memcached.default --channel "${BLDR_CHANNEL}" --force
}
export -f start_api

start_api_proxy() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-api-proxy" --bind http:builder-api.default --channel "${BLDR_CHANNEL}" --force
}
export -f start_api_proxy

start_datastore() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-datastore" --channel "${BLDR_CHANNEL}" --force
}
export -f start_datastore

start_minio() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-minio" --channel "${BLDR_CHANNEL}" --force
}
export -f start_minio

start_memcached() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-memcached" --channel "${BLDR_CHANNEL}" --force
}
export -f start_memcached

start_jobsrv() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-jobsrv" --channel stable --force
}
export -f start_jobsrv

start_worker() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-worker" --bind=jobsrv:builder-jobsrv.default --bind=depot:builder-api-proxy.default --channel stable --force
}
export -f start_worker

apply_db_password() {
  PW=$(cat /hab/svc/builder-datastore/config/pwfile)
  echo "datastore.password='$PW'" | sudo hab config apply builder-api.default "$(date +%s)"
  echo "datastore.password='$PW'" | sudo hab config apply builder-jobsrv.default "$(date +%s)"
}
export -f apply_db_password

generate_bldr_keys() {
  mapfile -t keys < <(find /hab/cache/keys -name "bldr-*.pub")

  if [ "${#keys[@]}" -gt 0 ]; then
    KEY_NAME=$(echo "${keys[0]}" | grep -Po "bldr-\\d+")
    echo "Re-using existing builder key: $KEY_NAME"
  else
    KEY_NAME=$(hab user key generate bldr | grep -Po "bldr-\\d+")
    echo "Generated new builder key: $KEY_NAME"
  fi

  hab file upload "builder-api.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.pub"
  hab file upload "builder-api.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.box.key"
}
export -f generate_bldr_keys

upload_ssl_certificate() {
  if [ "${APP_SSL_ENABLED}" = "true" ]; then
    echo "SSL enabled - uploading certificate files"
    if ! [ -f "../ssl-certificate.crt" ] || ! [ -f "../ssl-certificate.key" ]; then
      pwd
      echo "ERROR: Certificate file(s) not found!"
      exit 1
    fi
    hab file upload "builder-api-proxy.default" "$(date +%s)" "../ssl-certificate.crt"
    hab file upload "builder-api-proxy.default" "$(date +%s)" "../ssl-certificate.key"
  fi
}
export -f upload_ssl_certificate

start_init() {
  #  First shut down if already running
  echo "Stopping Habitat Supervisor"
  sudo hab sup term
  sleep 2

  echo "Removing builder spec files"
  sudo rm -rf /hab/sup/default/specs/builder-*

  echo "Starting Habitat Supervisor in test mode"
  create_users
  HAB_FUNC_TEST=1 hab sup run &
  sleep 2
}
export -f start_init

start_frontend() {
  echo
  echo "Starting Builder Frontend Services"
  start_memcached
  start_api
  start_api_proxy
}
export -f start_frontend

start_builder() {
  echo
  echo "Starting Builder Services"
  if [ "${PG_EXT_ENABLED:-false}" = "false" ]; then
    init_datastore
    start_datastore
    while [ ! -f /hab/svc/builder-datastore/config/pwfile ]
    do
      sleep 2
    done
    local pg_pass
    pg_pass=$(cat /hab/svc/builder-datastore/config/pwfile)
    cat <<EOT > pg_pass.toml
[datastore]
password = "$pg_pass"
EOT
  hab config apply builder-api.default "$(date +%s)" pg_pass.toml
  fi
  configure
  if [ "${ARTIFACTORY_ENABLED:-false}" = "false" ] && [ "${S3_ENABLED:-false}" = "false" ]; then
    start_minio
  fi
  start_frontend
  sleep 2
  generate_bldr_keys
  upload_ssl_certificate
}
export -f start_builder

create_users() {
  if command -v useradd > /dev/null; then
    sudo useradd --system --no-create-home hab || true
  else
    sudo adduser --system hab || true
  fi
  if command -v groupadd > /dev/null; then
    sudo groupadd --system hab || true
  else
    sudo addgroup --system hab || true
  fi
}
export -f create_users

wait_for_migrations() {
  echo "Waiting for migrations to finish"
  local count=0
  # while ! command with set -e fails on the first loop, so we get this slightly
  # more complex implementation
  while true; do
    # The status endpoint won't become available until migrations are finished
    if curl --silent --fail http://localhost:9636/v1/status; then
      break
    fi

    ((++count))
    if [ "$count" -ge 60 ]; then
      echo "--- Migrations failed to complete after one minute ---"
      exit 1
    fi
    sleep 1
  done
}
export -f wait_for_migrations
