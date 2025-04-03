#!/bin/bash

# This script exists to run our integration tests.

set -euo pipefail

export preserve_data
if [[ "${1:-}" == "-p" ]]; then
  preserve_data=true
else
  preserve_data=false
fi

echo "Executing ${BASH_SOURCE[0]} ${*}"

# make mocha happy by running from the directory it expects
cd "$(dirname "${BASH_SOURCE[0]}")"

clean_test_artifacts() {
  echo "Performing DB cleanup"
  local sql origins
  origins=( neurosis xmen umbrella deletemeifyoucan rcpd )

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
}

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

if npm run mocha; then
  echo "All tests passed"
  [[ "${preserve_data}" == false ]] && clean_test_artifacts
else
  mocha_exit_code=$?
  echo "Tests failed; skipping cleanup to facilitate investigation"
fi

exit ${mocha_exit_code:-0}
