#!/bin/bash

set -euo pipefail

export RUSTFLAGS="-D warnings"

# Because sadness
if ${BUILDKITE:-false}; then
  sudo chown buildkite-agent /home/buildkite-agent
fi

echo "--- loading shared build environment "
# shellcheck source=../support/ci/shared_build_environment.sh
source support/ci/shared_build_environment.sh
# shellcheck source=../support/ci/shared.sh
source support/ci/shared.sh

toolchain="${1:-"$(get_toolchain)"}"
install_rustup
install_rust_toolchain "$toolchain"

# Install clippy
echo "--- :rust: Installing clippy"
rustup component add --toolchain "$toolchain" clippy

# TODO: these should be in a shared script?
sudo hab license accept
install_hab_pkg core/rust/"$toolchain" core/libarchive core/openssl core/pkg-config core/zeromq core/patchelf core/cmake core/zlib
sudo hab pkg install --channel=LTS-2024 core/postgresql17
sudo hab pkg install --channel=LTS-2024 core/protobuf

# Yes, this is terrible but we need the clippy binary to run under our glibc.
# This became an issue with the latest refresh and can likely be dropped in
# the future when rust and supporting components are build against a later
# glibc.
sudo cp "$HOME"/.rustup/toolchains/"$toolchain"-x86_64-unknown-linux-gnu/bin/cargo-clippy "$(hab pkg path core/rust/"$toolchain")/bin"
sudo cp "$HOME"/.rustup/toolchains/"$toolchain"-x86_64-unknown-linux-gnu/bin/clippy-driver "$(hab pkg path core/rust/"$toolchain")/bin"
sudo hab pkg exec core/patchelf patchelf -- --set-interpreter "$(hab pkg path core/glibc)/lib/ld-linux-x86-64.so.2" "$(hab pkg path core/rust/"$toolchain")/bin/clippy-driver"
sudo hab pkg exec core/patchelf patchelf -- --set-interpreter "$(hab pkg path core/glibc)/lib/ld-linux-x86-64.so.2" "$(hab pkg path core/rust/"$toolchain")/bin/cargo-clippy"

export OPENSSL_NO_VENDOR=1
export LD_RUN_PATH
LD_RUN_PATH="$(hab pkg path core/glibc)/lib:$(hab pkg path core/gcc-libs)/lib:$(hab pkg path core/postgresql17)/lib:$(hab pkg path core/zeromq)/lib:$(hab pkg path core/libarchive)/lib"
export LD_LIBRARY_PATH
LD_LIBRARY_PATH="$(hab pkg path core/gcc-libs)/lib:$(hab pkg path core/zeromq)/lib:$(hab pkg path core/zlib)/lib"
export PKG_CONFIG_PATH
PKG_CONFIG_PATH="$(hab pkg path core/zeromq)/lib/pkgconfig:$(hab pkg path core/libarchive)/lib/pkgconfig:$(hab pkg path core/postgresql17)/lib/pkgconfig:$(hab pkg path core/openssl)/lib64/pkgconfig"

readonly OG_PATH=$PATH
eval "$(hab pkg env core/rust/"$toolchain")"
PATH="$PATH:$OG_PATH"
PATH="$(hab pkg path core/protobuf)/bin:$(hab pkg path core/pkg-config)/bin:$(hab pkg path core/postgresql17)/bin:$(hab pkg path core/cmake)/bin:$PATH"

# Lints we need to work through and decide as a team whether to allow or fix
mapfile -t unexamined_lints <"$2"

# Lints we disagree with and choose to keep in our code with no warning
mapfile -t allowed_lints <"$3"

# Known failing lints we want to receive warnings for, but not fail the build
mapfile -t lints_to_fix <"$4"

# Lints we don't expect to have in our code at all and want to avoid adding
# even at the cost of failing the build
mapfile -t denied_lints <"$5"

clippy_args=()

add_lints_to_clippy_args() {
  flag=$1
  shift
  for lint; do
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
cargo clippy --all-targets -- "${clippy_args[*]}"
