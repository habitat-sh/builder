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

# If we're in Buildkite, then install Rust, set up Habitat library
# dependencies, etc.
#
# If we're NOT in Buildkite, we'll just run clippy, assuming that
# the developer has already set up their environment as they like.
if ${BUILDKITE:-false}; then
    install_rustup
    install_rust_toolchain "$toolchain"

    # TODO: these should be in a shared script?
    sudo hab license accept
    install_hab_pkg core/bzip2 core/libarchive core/libsodium core/openssl core/xz core/zeromq core/libpq
    sudo hab pkg install core/protobuf --binlink

    export LIBARCHIVE_STATIC=true # so the libarchive crate *builds* statically
    export OPENSSL_DIR # so the openssl crate knows what to build against
    OPENSSL_DIR="$(hab pkg path core/openssl)"
    export OPENSSL_STATIC=true # so the openssl crate builds statically
    export LIBZMQ_PREFIX
    LIBZMQ_PREFIX=$(hab pkg path core/zeromq)
    # now include openssl and zeromq so thney exists in the runtime library path when cargo test is run
    export LD_LIBRARY_PATH
    LD_LIBRARY_PATH="$(hab pkg path core/libpq)/lib:$(hab pkg path core/libsodium)/lib:$(hab pkg path core/zeromq)/lib"
    # include these so that the cargo tests can bind to libarchive (which dynamically binds to xz, bzip, etc), openssl, and sodium at *runtime*
    export LIBRARY_PATH
    LIBRARY_PATH="$(hab pkg path core/libpq)/lib:$(hab pkg path core/bzip2)/lib:$(hab pkg path core/libsodium)/lib:$(hab pkg path core/openssl)/lib:$(hab pkg path core/xz)/lib"
    # setup pkgconfig so the libarchive crate can use pkg-config to fine bzip2 and xz at *build* time
    export PKG_CONFIG_PATH
    PKG_CONFIG_PATH="$(hab pkg path core/libpq)/lib/pkgconfig:$(hab pkg path core/libarchive)/lib/pkgconfig:$(hab pkg path core/libsodium)/lib/pkgconfig:$(hab pkg path core/openssl)/lib/pkgconfig"

    # Install clippy
    echo "--- :rust: Installing clippy"
    rustup component add clippy
fi

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
cargo +"$toolchain" clippy --all-targets --tests -- "${clippy_args[@]}"
