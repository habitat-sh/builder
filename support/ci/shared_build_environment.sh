#!/bin/bash

source ./support/ci/shared.sh

toolchain=$(get_toolchain)

sudo hab license accept

install_hab_pkg core/bash
install_hab_pkg core/binutils
install_hab_pkg core/cmake
install_hab_pkg core/coreutils
install_hab_pkg core/diffutils
install_hab_pkg core/gawk
install_hab_pkg core/gcc-base
install_hab_pkg core/git
install_hab_pkg core/glibc
install_hab_pkg core/grep
install_hab_pkg core/libarchive
install_hab_pkg core/libb2
install_hab_pkg core/libsodium
install_hab_pkg core/make
install_hab_pkg core/openssl
install_hab_pkg core/patchelf
install_hab_pkg core/pkg-config
install_hab_pkg core/postgresql17
install_hab_pkg core/protobuf
install_hab_pkg core/rust/"$toolchain"
install_hab_pkg core/sed
install_hab_pkg core/sudo
install_hab_pkg core/zeromq

# It is important NOT to use a vendored openssl from openssl-sys
# pg-sys does not use openssl-sys. So for components that use
# diesel's postgres feature, you wil end up with 2 versions of openssl
# which can lead to segmentation faults when connecting to postgres
export OPENSSL_NO_VENDOR=1

unset LD_RUN_PATH
export LD_RUN_PATH
LD_RUN_PATH="$(hab pkg path core/glibc)/lib"
LD_RUN_PATH+=":$(hab pkg path core/gcc-base)/lib64"
LD_RUN_PATH+=":$(hab pkg path core/libarchive)/lib"
LD_RUN_PATH+=":$(hab pkg path core/libb2)/lib"
LD_RUN_PATH+=":$(hab pkg path core/libsodium)/lib"
LD_RUN_PATH+=":$(hab pkg path core/openssl)/lib64"
LD_RUN_PATH+=":$(hab pkg path core/postgresql17)/lib"
LD_RUN_PATH+=":$(hab pkg path core/zeromq)/lib"
echo "LD_RUN_PATH: $LD_RUN_PATH"

unset LD_LIBRARY_PATH
export LD_LIBRARY_PATH
LD_LIBRARY_PATH="$(hab pkg path core/gcc-base)/lib64"
LD_LIBRARY_PATH+=":$(hab pkg path core/zeromq)/lib"
echo "LD_LIBRARY_PATH: $LD_LIBRARY_PATH"

export PKG_CONFIG_PATH
PKG_CONFIG_PATH="$(hab pkg path core/libarchive)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/libb2)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/libsodium)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/openssl)/lib64/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/postgresql17)/lib/pkgconfig"
PKG_CONFIG_PATH+=":$(hab pkg path core/zeromq)/lib/pkgconfig"
echo "PKG_CONFIG_PATH: $PKG_CONFIG_PATH"

export SODIUM_USE_PKG_CONFIG
SODIUM_USE_PKG_CONFIG=1

path="$(hab pkg path core/bash)/bin"
path+=":$(hab pkg path core/binutils)/bin"
path+=":$(hab pkg path core/coreutils)/bin"
path+=":$(hab pkg path core/diffutils)/bin"
path+=":$(hab pkg path core/gawk)/bin"
path+=":$(hab pkg path core/gcc-base)/bin"
path+=":$(hab pkg path core/git)/bin"
path+=":$(hab pkg path core/glibc)/bin"
path+=":$(hab pkg path core/grep)/bin"
path+=":$(hab pkg path core/libarchive)/bin"
path+=":$(hab pkg path core/hab)/bin"
path+=":$(hab pkg path core/make)/bin"
path+=":$(hab pkg path core/cmake)/bin"
path+=":$(hab pkg path core/openssl)/bin"
path+=":$(hab pkg path core/patchelf)/bin"
path+=":$(hab pkg path core/pkg-config)/bin"
path+=":$(hab pkg path core/postgresql17)/bin"
path+=":$(hab pkg path core/protobuf)/bin"
path+=":$(hab pkg path core/rust/"$toolchain")/bin"
path+=":$(hab pkg path core/sed)/bin"
path+=":$(hab pkg path core/sudo)/bin"
path+=":$(hab pkg path core/zeromq)/bin"
PATH=$path
echo "PATH: $PATH"
