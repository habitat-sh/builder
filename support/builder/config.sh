#!/bin/bash
set -euo pipefail

sleep 5

pwfile=/hab/svc/builder-datastore/config/pwfile
while [ ! -f $pwfile ] \
&&  hab sup status habitat/builder-datastore > /dev/null 2>&1;
do
  echo "Waiting for password"
  sleep 2
done

if [ -f $pwfile ]; then
  export PGPASSWORD
  PGPASSWORD=$(cat $pwfile)
else
  hab sup status
  echo "ERROR: $0: $pwfile does not exist and habitat/builder-datastore is not running"
  exit 1
fi

mkdir -p /hab/user/builder-minio/config
cat <<EOT > /hab/user/builder-minio/config/user.toml
key_id = "depot"
secret_key = "password"
EOT

mkdir -p /hab/user/builder-api/config
cat <<EOT > /hab/user/builder-api/config/user.toml
log_level = "debug,tokio_core=error,tokio_reactor=error,zmq=error,hyper=error"

[http]
handler_count = 15

[oauth]
provider = "$OAUTH_PROVIDER"
token_url = "$OAUTH_TOKEN_URL"
userinfo_url = "$OAUTH_USERINFO_URL"
redirect_url = "$OAUTH_REDIRECT_URL"
client_id = "$OAUTH_CLIENT_ID"
client_secret = "$OAUTH_CLIENT_SECRET"

[github]
api_url = "$GITHUB_API_URL"
app_id = $GITHUB_APP_ID

[datastore]
password = "$PGPASSWORD"
port = 5433
EOT

mkdir -p /hab/user/builder-api-proxy/config
cat <<EOT > /hab/user/builder-api-proxy/config/user.toml
log_level = "info"
enable_builder = true
enable_publisher_docker = true

app_url = "http://${APP_HOSTNAME}"

[github]
api_url = "$GITHUB_API_URL"
client_secret = "$OAUTH_CLIENT_SECRET"
app_id = $GITHUB_APP_ID
app_url = "${GITHUB_APP_URL}"

[oauth]
provider       = "$OAUTH_PROVIDER"
client_id      = "$OAUTH_CLIENT_ID"
authorize_url  = "$OAUTH_AUTHORIZE_URL"
redirect_url   = "$OAUTH_REDIRECT_URL"

[nginx]
max_body_size = "2048m"
proxy_send_timeout = 180
proxy_read_timeout = 180

[http]
keepalive_timeout = "180s"
EOT

mkdir -p /hab/user/builder-jobsrv/config/
cat <<EOT > /hab/user/builder-jobsrv/config/user.toml
log_level = "debug,tokio_core=error,tokio_reactor=error,zmq=error,postgres=error"

[http]
handler_count = 15

[datastore]
password = "$PGPASSWORD"
port = 5433

[archive]
backend = "local"
EOT

mkdir -p /hab/user/builder-worker/config
cat <<EOT > /hab/user/builder-worker/config/user.toml
log_level = "info"

key_dir = "/hab/svc/builder-worker/files"
auto_publish = true
airlock_enabled = false
data_path = "/hab/svc/builder-worker/data"
bldr_url = "http://localhost:9636"
work_poll_interval = 5

[github]
api_url = "$GITHUB_API_URL"
app_id = $GITHUB_APP_ID
EOT
