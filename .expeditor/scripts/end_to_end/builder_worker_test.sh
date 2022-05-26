#!/bin/bash

#set -euo pipefail

source .expeditor/scripts/post_habitat_release/shared.sh

export BLDR_CHANNEL=${HAB_BLDR_CHANNEL:=unstable}

export HAB_BLDR_URL=https://bldr.habitat.sh
export BLDR_ORIGIN=${HAB_BLDR_ORIGIN:=habitat}
#export BUILD_PKG_TARGET=x86_64-linux
export HAB_LICENSE=accept

curlbash_hab "${BUILD_PKG_TARGET:=x86_64-linux}"

echo "--- Generating signing key"
hab origin key generate ${BLDR_ORIGIN}

#  Clean out settings
sudo rm -rf /hab/user
sudo rm -rf /hab/svc

# Run the test
.expeditor/scripts/end_to_end/run_e2e.sh

echo "--- Terminating Habitat Supervisor"
# Terminate supervisor and remove specs files for builder
sudo hab sup term
sleep 2

echo "Remove builder spec files"
sudo rm -rf /hab/sup/default/specs/builder-*
