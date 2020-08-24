#!/bin/bash

set -euo pipefail

source .expeditor/scripts/post_habitat_release/shared.sh

export HAB_AUTH_TOKEN="${PIPELINE_HAB_AUTH_TOKEN}"
export HAB_BLDR_URL="${PIPELINE_HAB_BLDR_URL}"

########################################################################

# We have to install a kernel2 version of hab in order to build a
# kernel2 package; by default, these worker nodes are going to have
# the "normal" Linux package installed.
#
# However, given that this runs in a post-release pipeline, this is
# going to pull in the Habitat release that was just made, which means
# we might as well pull it in for *everything*. Otherwise, only the
# kernel2 package would be made with the new release, while the other
# platforms would be built with whatever (older) release comes with
# the container we happen to be running in.
curlbash_hab "${BUILD_PKG_TARGET}"

import_keys "${HAB_ORIGIN}"

echo "--- :habicat: Building builder-worker using $hab_binary "

${hab_binary} pkg build "components/builder-worker"
source results/last_build.env

if [ "${pkg_target}" != "${BUILD_PKG_TARGET}" ]; then
    echo "--- :face_with_symbols_on_mouth: Expected to build for target ${BUILD_PKG_TARGET}, but built ${pkg_target} instead!"
    exit 1
fi

echo "--- :habicat: Uploading ${pkg_ident:?} to ${HAB_BLDR_URL} in the 'unstable' channel"

${hab_binary} pkg upload \
    --auth="${HAB_AUTH_TOKEN}" \
    --no-build \
    "results/${pkg_artifact:?}"

echo "<br>* ${pkg_ident:?} (${BUILD_PKG_TARGET:?})" | buildkite-agent annotate --append --context "release-manifest"

set_worker_ident_for_target "${pkg_ident}" "${pkg_target}"
