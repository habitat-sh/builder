#!/bin/bash

set -eou pipefail

component=${1?component argument required}

source support/ci/shared_build_environment.sh

# Set testing filesystem root
export TESTING_FS_ROOT
TESTING_FS_ROOT=$(mktemp -d /tmp/testing-fs-root-XXXXXX)
cd "components/$component"
cargo test -- --nocapture
