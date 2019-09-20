#!/bin/bash

# This script exists to run our integration tests.

set -euo pipefail

# make mocha happy by running from the directory it expects
cd "$(dirname "${BASH_SOURCE[0]}")"

clean_test_artifacts() {
   local sql origins
  origins=( neurosis xmen umbrella )

  # clean origins
  local origins origin_tables
  origin_tables=( origin_integrations origin_project_integrations origin_secrets origin_private_encryption_keys origin_public_encryption_keys origin_members origin_channels origin_invitations origin_packages origin_projects origin_public_keys origin_secret_keys audit_package audit_package_group)
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
  users=( bobo mystique )
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
clean_test_artifacts # start with a clean slate

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
  echo "All tests passed, performing DB cleanup"
  clean_test_artifacts
else
  mocha_exit_code=$?
  echo "Tests failed; skipping cleanup to facilitate investigation"
fi

exit ${mocha_exit_code:-0}
