#!/bin/bash

set -euo pipefail

echo "--- Generating signing key"
hab origin key generate "$HAB_ORIGIN"

echo "--- Updating .studiorc" 
cat .expeditor/templates/studiorc >> .studiorc

echo "--- Copying habitat-env"
cp .secrets/habitat-env.sample .secrets/habitat-env

echo "--- Entering studio"
env HAB_INTERACTIVE=true \
    HAB_STUDIO_SUP=false \
    hab studio enter
