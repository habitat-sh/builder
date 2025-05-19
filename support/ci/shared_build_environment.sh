#!/bin/bash

sudo hab license accept

# shellcheck source=./shared.sh
source ./support/ci/shared.sh

OG_PATH=$PATH
toolchain=$(get_toolchain)
eval "$(hab pkg env core/rust/"$toolchain")"
PATH=$PATH:$OG_PATH
HAB_PATH="$(hab pkg path core/hab)/bin"

install_hab_pkg core/glibc
install_hab_pkg core/gcc-base
install_hab_pkg core/binutils
install_hab_pkg core/cmake
install_hab_pkg core/bash
install_hab_pkg core/coreutils
install_hab_pkg core/curl
install_hab_pkg core/diffutils
install_hab_pkg core/gawk
install_hab_pkg core/git
install_hab_pkg core/grep
install_hab_pkg core/libarchive
install_hab_pkg core/libb2
install_hab_pkg core/libsodium
install_hab_pkg core/make
install_hab_pkg core/openssl
install_hab_pkg core/pkg-config
install_hab_pkg core/postgresql17
install_hab_pkg core/protobuf
install_hab_pkg core/rust/"$toolchain"
install_hab_pkg core/sed
install_hab_pkg core/sudo
install_hab_pkg core/zeromq

# It is important NOT to use a vendored openssl from openssl-sys pg-sys does
# not use openssl-sys. So for components that use diesel's postgres feature,
# you wil end up with 2 versions of openssl which can lead to segmentation
# faults when connecting to postgres
export OPENSSL_NO_VENDOR=1

export OPENSSL_LIB_DIR
OPENSSL_LIB_DIR="$(hab pkg path core/openssl)/lib64"
export OPENSSL_INCLUDE_DIR
OPENSSL_INCLUDE_DIR="$(hab pkg path core/openssl)/include"

export SODIUM_USE_PKG_CONFIG=1

unset LD_RUN_PATH
export LD_RUN_PATH
LD_RUN_PATH="$(hab pkg path core/glibc)/lib"
LD_RUN_PATH+=":$(hab pkg path core/gcc-base)/lib64"
LD_RUN_PATH+=":$(hab pkg path core/binutils)/lib"
LD_RUN_PATH+=":$(hab pkg path core/libarchive)/lib"
LD_RUN_PATH+=":$(hab pkg path core/libb2)/lib"
LD_RUN_PATH+=":$(hab pkg path core/libsodium)/lib"
LD_RUN_PATH+=":$(hab pkg path core/openssl)/lib64"
LD_RUN_PATH+=":$(hab pkg path core/postgresql17)/lib"
LD_RUN_PATH+=":$(hab pkg path core/zeromq)/lib"
printf "\nLD_RUN_PATH='%s'\n" "${LD_RUN_PATH:-UNSET}"

if ${BUILDKITE:-false}; then
  unset LD_LIBRARY_PATH
  export LD_LIBRARY_PATH
  LD_LIBRARY_PATH="$(hab pkg path core/gcc-base)/lib64"
  # LD_LIBRARY_PATH="$(hab pkg path core/glibc)/lib"
  # LD_LIBRARY_PATH+=":$(hab pkg path core/binutils)/lib"
  # LD_LIBRARY_PATH+=":$(hab pkg path core/zeromq)/lib"

  # LD_PRELOAD="$(hab pkg path core/glibc)/lib/libc.so.6"
  # export LD_PRELOAD
fi
printf "\nLD_LIBRARY_PATH='%s'\n" "${LD_LIBRARY_PATH:-UNSET}"

unset PKG_CONFIG_PATH
export PKG_CONFIG_PATH
PKG_CONFIG_PATH="$(hab pkg path core/libarchive)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/bash)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/libb2)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/libsodium)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/openssl)/lib64/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/postgresql17)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/zeromq)/lib/pkgconfig"
printf "\nPKG_CONFIG_PATH='%s'\n" "${PKG_CONFIG_PATH:-UNSET}"

PATH=$HAB_PATH
export PATH
PATH+=":$(hab pkg path core/glibc)/bin"
PATH+=":$(hab pkg path core/gcc-base)/bin"
PATH+=":$(hab pkg path core/binutils)/bin"
PATH+=":$(hab pkg path core/bash)/bin"
PATH+=":$(hab pkg path core/coreutils)/bin"
PATH+=":$(hab pkg path core/curl)/bin"
PATH+=":$(hab pkg path core/diffutils)/bin"
PATH+=":$(hab pkg path core/gawk)/bin"
PATH+=":$(hab pkg path core/git)/bin"
PATH+=":$(hab pkg path core/grep)/bin"
PATH+=":$(hab pkg path core/libarchive)/bin"
PATH+=":$(hab pkg path core/make)/bin"
PATH+=":$(hab pkg path core/cmake)/bin"
PATH+=":$(hab pkg path core/openssl)/bin"
PATH+=":$(hab pkg path core/pkg-config)/bin"
PATH+=":$(hab pkg path core/postgresql17)/bin"
PATH+=":$(hab pkg path core/protobuf)/bin"
PATH+=":$(hab pkg path core/rust/"$toolchain")/bin"
PATH+=":$(hab pkg path core/sed)/bin"
PATH+=":$(hab pkg path core/sudo)/bin"
PATH+=":$(hab pkg path core/zeromq)/bin"
export PATH
printf "\nPATH='%s'\n\n" "${PATH:-UNSET}"
