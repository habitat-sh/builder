#!/bin/bash

set -euo pipefail

export RUSTFLAGS="-D warnings"

# Because sadness
if ${BUILDKITE:-false}; then
  sudo chown buildkite-agent /home/buildkite-agent
fi

source support/ci/shared_build_environment.sh
toolchain="${1:-"$(get_toolchain)"}"

echo "--- Patching clippy's glibc"
sudo hab pkg exec core/patchelf patchelf -- --set-interpreter "$(hab pkg path core/glibc)/lib/ld-linux-x86-64.so.2" "$(hab pkg path core/rust/"$toolchain")/bin/clippy-driver"
sudo hab pkg exec core/patchelf patchelf -- --set-interpreter "$(hab pkg path core/glibc)/lib/ld-linux-x86-64.so.2" "$(hab pkg path core/rust/"$toolchain")/bin/cargo-clippy"

# Lints we need to work through and decide as a team whether to allow or fix
mapfile -t unexamined_lints < "$2"

# Lints we disagree with and choose to keep in our code with no warning
mapfile -t allowed_lints < "$3"

# Known failing lints we want to receive warnings for, but not fail the build
mapfile -t lints_to_fix < "$4"

# Lints we don't expect to have in our code at all and want to avoid adding
# even at the cost of failing the build
mapfile -t denied_lints < "$5"

clippy_args=()

add_lints_to_clippy_args() {
  flag=$1
  shift
  for lint
  do
    clippy_args+=("$flag" "${lint}")
  done
}

set +u # See https://stackoverflow.com/questions/7577052/bash-empty-array-expansion-with-set-u/39687362#39687362
add_lints_to_clippy_args -A "${unexamined_lints[@]}"
add_lints_to_clippy_args -A "${allowed_lints[@]}"
add_lints_to_clippy_args -W "${lints_to_fix[@]}"
add_lints_to_clippy_args -D "${denied_lints[@]}"
set -u

echo "--- Running clippy!"
command="hab pkg exec core/rust/$toolchain cargo-clippy clippy --all-targets -vv -- ${clippy_args[*]}"
echo "Clippy rules: $command"
eval "$command"
