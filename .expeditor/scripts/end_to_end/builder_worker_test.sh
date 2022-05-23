#!/bin/bash

set -euo pipefail

source .expeditor/scripts/post_habitat_release/shared.sh

#  NOT WORKING
export HAB_AUTH_TOKEN="${PIPELINE_HAB_AUTH_TOKEN:=-}"
unset HAB_AUTH_TOKEN

export HAB_BLDR_URL=${PIPELINE_HAB_BLDR_URL:="https://bldr.habitat.sh"}
export BLDR_CHANNEL=${HAB_BLDR_CHANNEL:=unstable}
export BLDR_ORIGIN=${HAB_BLDR_ORIGIN:=habitat}
export BUILD_PKG_TARGET=x86_64-linux
export HAB_LICENSE=accept

curlbash_hab "${BUILD_PKG_TARGET}"

echo "--- Generating signing key"
hab origin key generate ${BLDR_ORIGIN}

#  Clean out user settings
sudo rm -rf /hab/user

# Start the builder services.
. .expeditor/scripts/end_to_end/run_e2e.sh ${BLDR_CHANNEL}
