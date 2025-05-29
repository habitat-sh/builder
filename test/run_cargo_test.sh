#!/bin/bash

set -eou pipefail

source ./support/ci/shared.sh

toolchain=$(get_toolchain)

component=${1?component argument required}

# shellcheck source=../support/ci/shared_build_environment.sh
source support/ci/shared_build_environment.sh

# Set testing filesystem root
export TESTING_FS_ROOT
TESTING_FS_ROOT=$(mktemp -d /tmp/testing-fs-root-XXXXXX)
cd "components/$component"
cargo test -- --nocapture
