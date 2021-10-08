#!/bin/bash

set -euo pipefail

# This is problematic if you want to be able to run this script from anywhere other than the root of the project,
# but changing it to an idiom like we have in rustfmt.sh breaks BK, so I dunno?
# shellcheck disable=SC1094
source ./support/ci/shared.sh

export RUSTFLAGS="-D warnings"

# Because sadness
if ${BUILDKITE:-false}; then
  sudo chown buildkite-agent /home/buildkite-agent
fi

toolchain="${1:-"$(get_toolchain)"}"
install_rustup
install_rust_toolchain "$toolchain"

# Install clippy
echo "--- :rust: Installing clippy"
rustup component add --toolchain "$toolchain" clippy

# TODO: these should be in a shared script?
sudo hab license accept
install_hab_pkg core/rust/"$toolchain" core/libarchive core/openssl core/pkg-config core/zeromq core/postgresql core/patchelf core/cmake
sudo hab pkg install core/protobuf

# Yes, this is terrible but we need the clippy binary to run under our glibc.
# This became an issue with the latest refresh and can likely be dropped in
# the future when rust and supporting components are build against a later
# glibc.
sudo cp "$HOME"/.rustup/toolchains/"$toolchain"-x86_64-unknown-linux-gnu/bin/cargo-clippy "$(hab pkg path core/rust/"$toolchain")/bin"
sudo cp "$HOME"/.rustup/toolchains/"$toolchain"-x86_64-unknown-linux-gnu/bin/clippy-driver "$(hab pkg path core/rust/"$toolchain")/bin"
sudo hab pkg exec core/patchelf patchelf -- --set-interpreter "$(hab pkg path core/glibc)/lib/ld-linux-x86-64.so.2" "$(hab pkg path core/rust/"$toolchain")/bin/clippy-driver"

export OPENSSL_NO_VENDOR=1
export LD_RUN_PATH
LD_RUN_PATH="$(hab pkg path core/glibc)/lib:$(hab pkg path core/gcc-libs)/lib:$(hab pkg path core/openssl)/lib:$(hab pkg path core/postgresql)/lib:$(hab pkg path core/zeromq)/lib:$(hab pkg path core/libarchive)/lib"
export LD_LIBRARY_PATH
LD_LIBRARY_PATH="$(hab pkg path core/gcc)/lib:$(hab pkg path core/zeromq)/lib"
export PKG_CONFIG_PATH
PKG_CONFIG_PATH="$(hab pkg path core/zeromq)/lib/pkgconfig:$(hab pkg path core/libarchive)/lib/pkgconfig:$(hab pkg path core/postgresql)/lib/pkgconfig:$(hab pkg path core/openssl)/lib/pkgconfig"
eval "$(hab pkg env core/rust/"$toolchain"):$(hab pkg path core/protobuf)/bin:$(hab pkg path core/pkg-config)/bin:$(hab pkg path core/postgresql)/bin:$(hab pkg path core/cmake)/bin:$PATH"

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
echo "Clippy rules: cargo clippy --all-targets --tests -- ${clippy_args[*]}"
cargo-clippy clippy --all-targets --tests -- "${clippy_args[@]}"
