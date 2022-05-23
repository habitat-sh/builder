#!/bin/bash

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

apply_db_password() {
  PW=$(cat /hab/svc/builder-datastore/config/pwfile)
  echo "datastore.password='$PW'" | sudo hab config apply builder-api.default $(date +%s)
  echo "datastore.password='$PW'" | sudo hab config apply builder-jobsrv.default $(date +%s)
}
export -f apply_db_password

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

start_jobsrv() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-jobsrv" --channel ${BLDR_CHANNEL} --force
}
export -f start_jobsrv

start_worker() {
  sudo hab svc load "${BLDR_ORIGIN}/builder-worker" --bind=jobsrv:builder-jobsrv.default --bind=depot:builder-api-proxy.default --channel ${BLDR_CHANNEL} --force
}
export -f start_worker

builder_api_reconfig() {
  PGUSER="hab"
  PGPASSWORD=""
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
  cat <<EOT > /hab/user/builder-api/config/user.toml
log_level="trace"
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
}
export -f builder_api_reconfig

