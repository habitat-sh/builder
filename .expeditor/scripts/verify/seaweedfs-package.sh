#!/bin/bash
# Buildkite entrypoint. Run the package test in a fresh Studio, without Builder.
set -euo pipefail

readonly channel=acceptance
mkdir -p logs/seaweedfs

# Match the Habitat tooling used by the existing Builder functional job.
for package in hab hab-studio hab-sup hab-launcher; do
  hab pkg install "chef/$package" --channel "$channel" --binlink --force
done
hab origin key generate "${HAB_ORIGIN:-habitat}"

env HAB_NONINTERACTIVE=true \
    HAB_STUDIO_SUP=false \
    HAB_INTERNAL_BLDR_CHANNEL="$channel" \
    HAB_STUDIO_SECRET_HAB_INTERNAL_BLDR_CHANNEL="$channel" \
    hab studio run 'bash /src/.expeditor/scripts/verify/seaweedfs-package-studio.sh'
