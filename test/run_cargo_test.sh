#!/bin/bash

set -eou pipefail

source ./support/ci/shared.sh

toolchain=$(get_toolchain)

component=${1?component argument required}

# Accept hab license
sudo hab license accept
sudo hab pkg install core/rust/"$toolchain"
sudo hab pkg install core/libarchive
sudo hab pkg install core/openssl
sudo hab pkg install core/zeromq
sudo hab pkg install core/pkg-config
sudo hab pkg install core/protobuf
sudo hab pkg install core/postgresql
# It is important NOT to use a vendored openssl from openssl-sys
# pg-sys does not use openssl-sys. So for components that use
# diesel's postgres feature, you wil end up with 2 versions of openssl
# which can lead to segmentation faults when connecting to postgres
export OPENSSL_NO_VENDOR=1
export LD_RUN_PATH
LD_RUN_PATH="$(hab pkg path core/glibc)/lib:$(hab pkg path core/gcc-libs)/lib:$(hab pkg path core/openssl)/lib:$(hab pkg path core/postgresql)/lib:$(hab pkg path core/zeromq)/lib:$(hab pkg path core/libarchive)/lib"
export PKG_CONFIG_PATH
PKG_CONFIG_PATH="$(hab pkg path core/zeromq)/lib/pkgconfig:$(hab pkg path core/libarchive)/lib/pkgconfig:$(hab pkg path core/postgresql)/lib/pkgconfig:$(hab pkg path core/openssl)/lib/pkgconfig"
eval "$(hab pkg env core/rust/"$toolchain"):$(hab pkg path core/protobuf)/bin:$(hab pkg path core/pkg-config)/bin:$(hab pkg path core/postgresql)/bin:$PATH"

# Set testing filesystem root
export TESTING_FS_ROOT
TESTING_FS_ROOT=$(mktemp -d /tmp/testing-fs-root-XXXXXX)
cd "components/$component"
cargo test -- --nocapture
