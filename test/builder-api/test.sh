#!/bin/bash

# This script exists to run our integration tests.

set -x
set -euo pipefail

# make mocha happy by running from the directory it expects
cd "$(dirname "${BASH_SOURCE[0]}")"

clean_test_artifacts() {
  origins=( neurosis xmen )
  origin_id_tables=( origin_members origin_channels origin_invitations origin_packages origin_projects origin_public_keys origin_secret_keys )
  origin_tables=( origin_integrations origin_project_integrations )

  for origin in "${origins[@]}"; do
    sql+="SET SEARCH_PATH TO shard_$(op shard "$origin");"
    sql+="DELETE FROM origin_channel_packages WHERE channel_id IN (SELECT id FROM origin_channels WHERE origin_id=(SELECT id FROM origins WHERE name='$origin'));"

    for table in "${origin_id_tables[@]}"; do
      sql+="DELETE FROM $table WHERE origin_id=(SELECT id FROM origins WHERE name='$origin');"
    done

    for table in "${origin_tables[@]}"; do
      sql+="DELETE FROM $table WHERE origin='$origin';"
    done

    sql+="DELETE FROM origins WHERE name='$origin';"
  done

  psql builder_originsrv -q -c "$sql"
}

clean_test_artifacts # start with a clean slate

if npm run mocha; then
  echo "All tests passed, performing DB cleanup"
  clean_test_artifacts
else
  mocha_exit_code=$?
  echo "Tests failed; skipping cleanup to facilitate investigation"
fi

exit ${mocha_exit_code:-0}
