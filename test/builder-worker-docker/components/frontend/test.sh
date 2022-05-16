#!/bin/bash

# This script exists to run our integration tests.

set -euo pipefail

echo "Executing ${BASH_SOURCE[0]} ${*}"

# make mocha happy by running from the directory it expects
cd "$(dirname "${BASH_SOURCE[0]}")"

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

wait_for_migrations

# start with a clean slate
clean_test_artifacts

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

exit ${mocha_exit_code:-0}
