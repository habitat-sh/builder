#!/bin/bash

set -euo pipefail

component=builder-api

# shellcheck source=.expeditor/scripts/post_habitat_release/shared.sh
source .expeditor/scripts/post_habitat_release/shared.sh

echo "--- Generating Habitat Builder API docs"

#  Check if hub is installed and install if not
hub_check=$(which hub)
if [ -z "$hub_check" ]; then
  install_hub
fi

hab pkg install core/node

npm install webapi-parser@0.5.0
npm install minimist@1.2.5

tempdir="$(mktemp --directory --tmpdir="$(pwd)" -t "docs-XXXX")"
cd "${tempdir}"

git clone "https://github.com/habitat-sh/habitat.git"
cd ..

#  Generate the api docs file. 
input_file=components/${component}/doc/api.raml
output_file=${tempdir}/${component}".json"

repo_file=${tempdir}/habitat/components/docs-chef-io/static/habitat-api-docs/${component}".json"

node .expeditor/scripts/release_builder/hab-raml-converter.js -i "${input_file}" -o "${output_file}"

#  Only proceed with pull request if it has changed.
if cmp -s "${output_file}" "${repo_file}"; then
  echo "Habitat Builder API docs generation is unnecessary"
  echo "Removing temp directory"
  rm -rf "${tempdir}"
  exit 0
else
  echo "Habitat Builder API docs generation is necessary"
fi

cd "${tempdir}/habitat"

cp "${output_file}" "${repo_file}"

TIMESTAMP=$(date '+%Y%m%d%H%M%S')
readonly branch="expeditor/habitat_release_$TIMESTAMP"
git checkout -b "${branch}"

echo "--- :git: Pushing new branch ${branch}"
git add "${repo_file}"
git commit --signoff --message "Update Habitat Builder API Docs - ${TIMESTAMP}"
push_current_branch

echo "--- :github: Creating PR"
hub pull-request --force --no-edit --message "Update Habitat Builder API Docs - ${TIMESTAMP}"

echo "Removing temp directory"
rm -rf "${tempdir}"
