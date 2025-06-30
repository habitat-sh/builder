#!/bin/bash

sudo hab license accept

# shellcheck source=./shared.sh
source ./support/ci/shared.sh

readonly OG_PATH=$PATH
toolchain=$(get_toolchain)
sudo -E hab pkg install core/rust/"$toolchain" --force
eval "$(hab pkg env core/rust/"$toolchain")"
PATH=$PATH:$OG_PATH

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
install_hab_pkg core/hab-ld-wrapper
install_hab_pkg core/libarchive
install_hab_pkg core/libb2
install_hab_pkg core/libsodium
install_hab_pkg core/make
install_hab_pkg core/openssl
install_hab_pkg core/pkg-config
install_hab_pkg core/postgresql17
install_hab_pkg core/protobuf
install_hab_pkg core/sed
install_hab_pkg core/sudo
install_hab_pkg core/zeromq
install_hab_pkg core/zlib

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
LD_RUN_PATH+=":$(hab pkg path core/zlib)/lib"
printf "\nLD_RUN_PATH='%s'\n" "${LD_RUN_PATH:-UNSET}"

unset LD_LIBRARY_PATH
export LD_LIBRARY_PATH
LD_LIBRARY_PATH="$(hab pkg path core/gcc-base)/lib64"
LD_LIBRARY_PATH+=":$(hab pkg path core/zlib)/lib"
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
PKG_CONFIG_PATH+=":$(hab pkg path core/zlib)/lib/pkgconfig"
printf "\nPKG_CONFIG_PATH='%s'\n" "${PKG_CONFIG_PATH:-UNSET}"

prepend_path="$(hab pkg path core/glibc)/bin"
prepend_path+=":$(hab pkg path core/gcc-base)/bin"
prepend_path+=":$(hab pkg path core/binutils)/bin"
prepend_path+=":$(hab pkg path core/bash)/bin"
prepend_path+=":$(hab pkg path core/coreutils)/bin"
prepend_path+=":$(hab pkg path core/curl)/bin"
prepend_path+=":$(hab pkg path core/diffutils)/bin"
prepend_path+=":$(hab pkg path core/gawk)/bin"
prepend_path+=":$(hab pkg path core/git)/bin"
prepend_path+=":$(hab pkg path core/grep)/bin"
prepend_path+=":$(hab pkg path core/hab-ld-wrapper)/bin"
prepend_path+=":$(hab pkg path core/libarchive)/bin"
prepend_path+=":$(hab pkg path core/make)/bin"
prepend_path+=":$(hab pkg path core/cmake)/bin"
prepend_path+=":$(hab pkg path core/openssl)/bin"
prepend_path+=":$(hab pkg path core/pkg-config)/bin"
prepend_path+=":$(hab pkg path core/postgresql17)/bin"
prepend_path+=":$(hab pkg path core/protobuf)/bin"
prepend_path+=":$(hab pkg path core/rust/"$toolchain")/bin"
prepend_path+=":$(hab pkg path core/sed)/bin"
prepend_path+=":$(hab pkg path core/sudo)/bin"
prepend_path+=":$(hab pkg path core/zeromq)/bin"
PATH=$prepend_path:$PATH
export PATH
unset prepend_path
printf "\nPATH='%s'\n\n" "${PATH:-UNSET}"
