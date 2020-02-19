#!/bin/bash

set -euo pipefail 
 
# shellcheck source=.expeditor/scripts/shared.sh 
source .expeditor/scripts/post_habitat_release/shared.sh 

branch="ci/cargo-update-$(date +"%Y%m%d%H%M%S")"
git checkout -b "$branch"

toolchain="$(get_toolchain)"

install_rustup
rustup install "$toolchain"

install_hub

echo "--- :habicat: Installing and configuring build dependencies"
hab pkg install core/libarchive \
                core/libsodium \
                core/openssh \
                core/openssl \
                core/pkg-config \
                core/postgresql-client \
                core/protobuf \
                core/zeromq

PKG_CONFIG_PATH="$(< "$(hab pkg path core/libarchive)"/PKG_CONFIG_PATH)"
PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(< "$(hab pkg path core/libsodium)"/PKG_CONFIG_PATH)"
PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(< "$(hab pkg path core/openssl)"/PKG_CONFIG_PATH)"
PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$(< "$(hab pkg path core/zeromq)"/PKG_CONFIG_PATH)"
export PKG_CONFIG_PATH

# The library detection for the zeromq crate needs this additional hint.
LD_RUN_PATH="$(< "$(hab pkg path core/zeromq)"/LD_RUN_PATH)"
export LD_RUN_PATH

# 'protoc' needs to be on our path
PATH="$PATH:$(< "$(hab pkg path core/protobuf)"/RUNTIME_PATH)"
# The the default container we use in buildkite has pg_config in /usr/bin which 
# causes the pq-sys crate to be unable to set the correct path to the libraries.
# This doesn't manifest until later in the build process in the migrations-macros
# crate which is unable to find libpq. Place postgresql-client on our path first.
PATH="$(< "$(hab pkg path core/postgresql-client)"/RUNTIME_PATH):$PATH"
export PATH

echo "--- :rust: Cargo Update"
cargo clean
cargo +"$toolchain" update

echo "--- :rust: Cargo Check"
cargo +"$toolchain" check --all --tests && update_status=$? || update_status=$?

echo "--- :git: Publishing updated Cargo.lock"
git add Cargo.lock

git commit -s -m "Update Cargo.lock"

pr_labels=""
pr_message=""
if [ "$update_status" -ne 0 ]; then 
  pr_labels="T-DO-NOT-MERGE"

  # read will exit 1 if it can't find a delimeter.
  # -d '' will always trigger this case as there is no delimeter to find, 
  # but this is required in order to write the entire message into a single PR 
  # preserving newlines.
   read -r -d '' pr_message <<EOM || true
Unable to update Cargo.lock!

For details on the failure, please visit ${BUILDKITE_BUILD_URL:-No Buildkite url}#${BUILDKITE_JOB_ID:-No Buildkite job id}
EOM
fi

# Use shell to push the current branch so we can authenticate without scribbling secrets to disk
#   or being prompted by git.
# Hub provides better mechanisms than curl for opening a pr with a message and setting labels,
# the latter requires multiple curl commands and parsing json responses and error handling at each step.
push_current_branch

# We have to use --force to open the PR. We're specifying where to push, rather than using a remote, in 
# the previous command to avoid writing secrets to disk, so hub isn't able to read that information from
# the git configuration
hub pull-request --force --no-edit --draft --labels "$pr_labels" --file - <<EOF
Cargo Update

$pr_message
EOF

exit "$update_status"
