#!/bin/bash

set -euo pipefail

readonly channel='acceptance'
readonly storage_backend=${1:-minio}
case "$storage_backend" in
  minio|seaweedfs) ;;
  *) echo "Usage: $0 [minio|seaweedfs]" >&2; exit 1 ;;
esac

# 10/11/2024: We need to use the most recent hab binary that is not yet in stable
# to build against LTS without conflicts with existing stable packages
hab pkg install chef/hab --channel="${channel}" --binlink --force

# 2025-07-16: Installing these works around a bug that exists at the
# current time where the HAB_AUTH_TOKEN isn't read  when packages are
# automatically installed by calls to different hab commands.
# See https://progresssoftware.atlassian.net/browse/CHEF-23525
hab pkg install chef/hab-studio --channel="${channel}" --binlink --force
hab pkg install chef/hab-sup --channel="${channel}" --binlink --force
hab pkg install chef/hab-launcher --channel="${channel}" --binlink --force

echo "--- Generating signing key"
hab origin key generate "$HAB_ORIGIN"

echo "--- Updating .studiorc"
cat .expeditor/templates/studiorc >>.studiorc

echo "--- Copying habitat-env"
cp .secrets/habitat-env.sample .secrets/habitat-env

echo "--- Entering studio"
env HAB_NONINTERACTIVE=true \
    HAB_STUDIO_SUP=false \
    HAB_INTERNAL_BLDR_CHANNEL="${channel}" \
    HAB_STUDIO_SECRET_HAB_INTERNAL_BLDR_CHANNEL="${channel}" \
    HAB_STUDIO_SECRET_BUILDER_TEST_STORAGE_BACKEND="$storage_backend" \
    hab studio enter
