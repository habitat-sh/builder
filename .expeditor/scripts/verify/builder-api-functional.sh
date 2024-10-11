#!/bin/bash

set -euo pipefail

# 10/11/2024: We need to use the most recent hab binary that is not yet in stable
# to build against LTS without conflicts with existing stable packages
hab pkg install core/hab --channel acceptance -bf

echo "--- Generating signing key"
hab origin key generate "$HAB_ORIGIN"

echo "--- Updating .studiorc" 
cat .expeditor/templates/studiorc >> .studiorc

echo "--- Copying habitat-env"
cp .secrets/habitat-env.sample .secrets/habitat-env

echo "--- Entering studio"
env HAB_NONINTERACTIVE=true \
    HAB_STUDIO_SUP=false \
    HAB_INTERNAL_BLDR_CHANNEL=acceptance \
    HAB_STUDIO_SECRET_HAB_INTERNAL_BLDR_CHANNEL=acceptance \
    hab studio enter
