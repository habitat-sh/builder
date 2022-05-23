#!/bin/bash

set -eou pipefail
umask 0022 

#  Always run from builder root directory.
export ROOT_DIR=`pwd`
WORK_DIR=test/end-to-end/worker-test

source ${WORK_DIR}/bldr-end-to-end.env
source ${WORK_DIR}/shared.sh

sudo () {
  [[ $EUID = 0 ]] || set -- command sudo -E "$@"
  "$@"
}

# Defaults
BLDR_ORIGIN=${BLDR_ORIGIN:="habitat"}

#export HAB_FUNC_TEST=1

#  Assumes hab-sup is installed as a service
sudo systemctl stop hab-sup.service

#  Getting database corruption
sudo rm -rf /hab/svc/builder-datastore

cd ${WORK_DIR}
git clone https://github.com/habitat-sh/on-prem-builder.git
cp bldr-end-to-end.env on-prem-builder/bldr.env
cd on-prem-builder
sudo ./scripts/provision.sh

sleep 1
cd ..
sudo rm -rf on-prem-builder

cd ${ROOT_DIR}

#  Reconfig the builder api with jobsrv enabled.
builder_api_reconfig

start_jobsrv
start_worker

apply_db_password

cat <<EOT > /tmp/builder_worker.toml
log_level='trace'
github.app_id = 8053
github.webhook_secret=''
EOT

hab config apply builder-worker.default $(date +%s) /tmp/builder_worker.toml

sleep 3

######   TO BE REMOVED ######
#  To get builder-worker HAB_FUNC_TEST changes 
#hab svc stop habitat/builder-worker
#sleep 5
#cp ${WORK_DIR}/bldr-worker /hab/pkgs/habitat/builder-worker/10041/20220510143824/bin

#sleep 2
#hab svc start habitat/builder-worker

#  Jobsrv debugging
#hab svc stop habitat/builder-jobsrv
#sleep 2
#cp ${WORK_DIR}/bldr-jobsrv /hab/pkgs/habitat/builder-jobsrv/9982/20211222144544/bin
#hab svc start habitat/builder-jobsrv

######   TO BE REMOVED ######

while hab svc status | grep --quiet down;
do 
  sleep 5
done

echo "Services have started - continuing with tests"
sudo cp ${WORK_DIR}/builder-github-app.pem /hab/svc/builder-worker/files
sudo cp ${WORK_DIR}/neurosis-20171211220037.pub /hab/svc/builder-worker/files
sudo cp ${WORK_DIR}/neurosis-20171211220037.sig.key /hab/svc/builder-worker/files

echo "Begin waiting for migrations $(date +%s)"
sleep 120
wait_for_migrations

echo "Done waiting for migrations $(date +%s)"

clean_test_artifacts

#  Install packages
hab pkg install core/openssl
hab pkg install core/node --binlink
hab pkg install core/coreutils

cd ${WORK_DIR}
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

if ! [ -d node_modules/dotenv ]; then
  npm install dotenv
fi

if npm run mocha; then
  echo "Setup tests passed"
else
  mocha_exit_code=$?
  echo "Setup tests failed"
fi

cd ${ROOT_DIR}

exit ${mocha_exit_code:-0}
