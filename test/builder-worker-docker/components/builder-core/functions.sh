#!/bin/bash

configure_builder() {
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
    FEATURES_ENABLED="ARTIFACTORY"
  else
    FEATURES_ENABLED=""
    ARTIFACTORY_API_URL="http://localhost:8081"
    ARTIFACTORY_API_KEY="none"
    ARTIFACTORY_REPO="habitat-builder-artifact-store"
  fi
  PG_HOST=${POSTGRES_HOST:-localhost}
  PG_PORT=${POSTGRES_PORT:-5432}
  cat <<EOT > /hab/user/builder-api/config/user.toml
log_level="error,tokio_core=error,tokio_reactor=error,zmq=error,hyper=error"
jobsrv_enabled = true

[http]
handler_count = 10

[api]
features_enabled = "jobsrv"
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
mkdir -p /hab/user/builder-api-proxy/config
cat <<EOT > /hab/user/builder-api-proxy/config/user.toml
log_level="info"
enable_builder = false
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
export -f configure_builder

init_datastore() {
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

configure_bldr_keys() {
  KEY_NAME=$(hab user key generate bldr | grep -Po "bldr-\\d+")

  hab file upload "builder-api.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.pub"
  hab file upload "builder-api.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.box.key"
  hab file upload "builder-jobsrv.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.pub"
  hab file upload "builder-jobsrv.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.box.key"
  hab file upload "builder-worker.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.pub"
  hab file upload "builder-worker.default" "$(date +%s)" "/hab/cache/keys/${KEY_NAME}.box.key"

  mkdir -p /hab/svc/builder-worker/files
  cp /hab/cache/keys/${KEY_NAME}.pub /hab/svc/builder-worker/files
  cp /hab/cache/keys/${KEY_NAME}.box.key /hab/svc/builder-worker/files

  cp ./builder-github-app.pem /hab/svc/builder-api/files
  cp ./builder-github-app.pem /hab/svc/builder-jobsrv/files
  cp ./builder-github-app.pem /hab/svc/builder-worker/files
  hab file upload "builder-api.default" "$(date +%s)" "./builder-github-app.pem"
  hab file upload "builder-jobsrv.default" "$(date +%s)" "./builder-github-app.pem"
  hab file upload "builder-worker.default" "$(date +%s)" "./builder-github-app.pem"
  echo "${KEY_NAME}"
}
export -f configure_bldr_keys

start_bldr_services() {
hab svc load "${HAB_ORIGIN}/builder-datastore" --channel "${CHANNEL}" --force
hab svc load "${HAB_ORIGIN}/builder-memcached" --channel "${CHANNEL}" --force
hab svc load "${HAB_ORIGIN}/builder-api" --bind=memcached:builder-memcached.default --channel "${CHANNEL}" --force

configure_builder

hab svc load "${HAB_ORIGIN}/builder-api-proxy" --bind=http:builder-api.default --channel "${CHANNEL}" --force

hab svc load "${HAB_ORIGIN}/builder-jobsrv" --channel "${CHANNEL}" --force
hab svc load "${HAB_ORIGIN}/builder-minio" --channel "${CHANNEL}" --force
hab svc load "${HAB_ORIGIN}/builder-worker" --bind=jobsrv:builder-jobsrv.default --bind=depot:builder-api-proxy.default --channel "${CHANNEL}" --force

while [ ! -f /hab/svc/builder-datastore/config/pwfile ]
do
  echo "NO PASSWORD NO PASSWORD NO PASSWORD"
  sleep 2
done
pg_pass=$(cat /hab/svc/builder-datastore/config/pwfile)
cat <<EOT > pg_pass.toml
[datastore]
password = "$pg_pass"
EOT

hab config apply builder-api.default $(date +%s) pg_pass.toml
hab config apply builder-jobsrv.default $(date +%s) pg_pass.toml
echo "log_level='trace'" | hab config apply builder-worker.default $(date +%s)

export PGPASSWORD=$(cat /hab/svc/builder-datastore/config/pwfile)
echo "PGPASSWORD=${PGPASSWORD}"

configure_bldr_keys
}
export -f start_bldr_services
