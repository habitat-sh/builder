#!/bin/bash
set -euo pipefail

pwfile=/hab/svc/builder-datastore/config/pwfile
while [ ! -f $pwfile ] \
&&    hab sup status habitat/builder-datastore > /dev/null
do
  sleep 2
done

if [ -f $pwfile ]; then
  export PGPASSWORD
  PGPASSWORD=$(cat $pwfile)
else
  echo "ERROR: $0: $pwfile does not exist and habitat/builder-datastore is not running"
  exit 1
fi

mkdir -p /hab/svc/builder-minio
cat <<EOT > /hab/svc/builder-minio/user.toml
key_id = "depot"
secret_key = "password"
EOT

mkdir -p /hab/svc/builder-router
cat <<EOT > /hab/svc/builder-router/user.toml
log_level = "info"
EOT

mkdir -p /hab/svc/builder-api
cat <<EOT > /hab/svc/builder-api/user.toml
log_level = "debug,tokio_core=error,tokio_reactor=error"

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
EOT

mkdir -p /hab/svc/builder-api-proxy
cat <<EOT > /hab/svc/builder-api-proxy/user.toml
log_level = "info"
enable_builder = true

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

mkdir -p /hab/svc/builder-jobsrv
cat <<EOT > /hab/svc/builder-jobsrv/user.toml
log_level = "info"

[datastore]
password = "$PGPASSWORD"
database = "builder_jobsrv"

[archive]
backend = "local"
EOT

mkdir -p /hab/svc/builder-originsrv
cat <<EOT > /hab/svc/builder-originsrv/user.toml
log_level = "info"

[app]
shards = [
  0,
  1,
  2,
  3,
  4,
  5,
  6,
  7,
  8,
  9,
  10,
  11,
  12,
  13,
  14,
  15,
  16,
  17,
  18,
  19,
  20,
  21,
  22,
  23,
  24,
  25,
  26,
  27,
  28,
  29,
  30,
  31,
  32,
  33,
  34,
  35,
  36,
  37,
  38,
  39,
  40,
  41,
  42,
  43,
  44,
  45,
  46,
  47,
  48,
  49,
  50,
  51,
  52,
  53,
  54,
  55,
  56,
  57,
  58,
  59,
  60,
  61,
  62,
  63,
  64,
  65,
  66,
  67,
  68,
  69,
  70,
  71,
  72,
  73,
  74,
  75,
  76,
  77,
  78,
  79,
  80,
  81,
  82,
  83,
  84,
  85,
  86,
  87,
  88,
  89,
  90,
  91,
  92,
  93,
  94,
  95,
  96,
  97,
  98,
  99,
  100,
  101,
  102,
  103,
  104,
  105,
  106,
  107,
  108,
  109,
  110,
  111,
  112,
  113,
  114,
  115,
  116,
  117,
  118,
  119,
  120,
  121,
  122,
  123,
  124,
  125,
  126,
  127
]

[datastore]
password = "$PGPASSWORD"
database = "builder_originsrv"
EOT

mkdir -p /hab/svc/builder-sessionsrv
cat <<EOT > /hab/svc/builder-sessionsrv/user.toml
log_level = "info"

[app]
shards = [
  0,
  1,
  2,
  3,
  4,
  5,
  6,
  7,
  8,
  9,
  10,
  11,
  12,
  13,
  14,
  15,
  16,
  17,
  18,
  19,
  20,
  21,
  22,
  23,
  24,
  25,
  26,
  27,
  28,
  29,
  30,
  31,
  32,
  33,
  34,
  35,
  36,
  37,
  38,
  39,
  40,
  41,
  42,
  43,
  44,
  45,
  46,
  47,
  48,
  49,
  50,
  51,
  52,
  53,
  54,
  55,
  56,
  57,
  58,
  59,
  60,
  61,
  62,
  63,
  64,
  65,
  66,
  67,
  68,
  69,
  70,
  71,
  72,
  73,
  74,
  75,
  76,
  77,
  78,
  79,
  80,
  81,
  82,
  83,
  84,
  85,
  86,
  87,
  88,
  89,
  90,
  91,
  92,
  93,
  94,
  95,
  96,
  97,
  98,
  99,
  100,
  101,
  102,
  103,
  104,
  105,
  106,
  107,
  108,
  109,
  110,
  111,
  112,
  113,
  114,
  115,
  116,
  117,
  118,
  119,
  120,
  121,
  122,
  123,
  124,
  125,
  126,
  127
]

[datastore]
password = "$PGPASSWORD"
database = "builder_sessionsrv"
EOT

mkdir -p /hab/svc/builder-worker
cat <<EOT > /hab/svc/builder-worker/user.toml
log_level = "info"

key_dir = "/hab/svc/builder-worker/files"
auto_publish = true
airlock_enabled = false
data_path = "/hab/svc/builder-worker/data"
bldr_url = "http://localhost:9636"

[github]
api_url = "$GITHUB_API_URL"
app_id = $GITHUB_APP_ID
EOT
