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

if [[ -d "components/$component" ]]; then
  cd "components/$component"
elif [[ -d "tools/$component" ]]; then
  cd "tools/$component"
else
  echo "Unknown cargo package path for: $component" >&2
  exit 1
fi

cargo test -- --nocapture
