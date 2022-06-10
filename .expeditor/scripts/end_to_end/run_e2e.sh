#!/bin/bash

set -eou pipefail
umask 0022 

#  Always run from builder root directory.
export ROOT_DIR
ROOT_DIR=$(pwd)
WORK_DIR=${ROOT_DIR}/test/end-to-end/worker-test

source "${WORK_DIR}"/bldr-end-to-end.env
source "${WORK_DIR}"/shared.sh

export HAB_FUNC_TEST=1

start_init

start_builder

start_jobsrv

apply_db_password
sleep 5

sudo cp "${WORK_DIR}"/builder-github-app.pem /hab/svc/builder-api/files

echo "start_worker"
start_worker

cat <<EOT > /tmp/builder_worker.toml
log_level='trace'
github.app_id = 8053
github.webhook_secret=''
EOT

hab config apply builder-worker.default "$(date +%s)" /tmp/builder_worker.toml
sleep 3
echo "worker config updated"
sudo cp "${WORK_DIR}"/builder-github-app.pem /hab/svc/builder-worker/files

#  HACK  To get builder-worker HAB_FUNC_TEST changes 
#hab svc stop habitat/builder-worker
#sleep 5
#cp ${WORK_DIR}/bldr-worker /hab/pkgs/habitat/builder-worker/10041/20220510143824/bin
#
#sleep 2
#hab svc start habitat/builder-worker
#  HACK  To get builder-worker HAB_FUNC_TEST changes 

while hab svc status | grep --quiet down;
do 
  sleep 5
done

echo "Services have started - continuing with tests"

#  Need to test if we need this file in this location and remove if not.
sudo cp "${WORK_DIR}"/fixtures/neurosis-20171211220037.pub /hab/svc/builder-worker/files
sudo cp "${WORK_DIR}"/fixtures/neurosis-20171211220037.sig.key /hab/svc/builder-worker/files

sudo cp /hab/svc/builder-api/files/bldr-*.pub /hab/svc/builder-worker/files
sudo cp /hab/svc/builder-api/files/bldr-*.box.key /hab/svc/builder-worker/files

wait_for_migrations

clean_test_artifacts

#  Install packages
hab pkg install core/openssl
hab pkg install core/node --binlink
hab pkg install core/coreutils

cd test/end-to-end/worker-test
npm install mocha

if ! command -v npm >/dev/null 2>&1; then
  hab pkg install core/node -b
fi

if ! [ -f /usr/bin/env ]; then
  hab pkg binlink core/coreutils -d /usr/bin env
fi

if ! [ -d node_modules/mocha ]; then
  npm install mocha
fi

if ! [ -d node_modules/chai ]; then
  npm install chai
fi

if ! [ -d node_modules/supertest ]; then
  npm install supertest
fi

if ! [ -d node_modules/superagent-binary-parser ]; then
  npm install superagent-binary-parser
fi

if npm run mocha; then
  echo "Setup tests passed"
else
  mocha_exit_code=$?
  echo "Setup tests failed"
fi

echo "Exiting run script"
exit ${mocha_exit_code:-0}
