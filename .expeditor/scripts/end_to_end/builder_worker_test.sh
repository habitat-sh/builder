#!/bin/bash

set -euo pipefail

export HAB_BLDR_URL=${PIPELINE_HAB_BLDR_URL:="https://bldr.habitat.sh"}
export BLDR_CHANNEL=${HAB_BLDR_CHANNEL:=unstable}
export BLDR_ORIGIN=${HAB_BLDR_ORIGIN:=habitat}

echo "--- Generating signing key"
hab origin key generate ${BLDR_ORIGIN}

#  Clean out user settings
sudo rm -rf /hab/user

# Start the builder services.
. .expeditor/scripts/end_to_end/run_e2e.sh ${BLDR_CHANNEL}
