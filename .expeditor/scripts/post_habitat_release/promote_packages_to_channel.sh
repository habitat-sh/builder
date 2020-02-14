#!/bin/bash

set -euo pipefail

promotion_channel="${1}"

source .expeditor/scripts/post_habitat_release/shared.sh

export HAB_AUTH_TOKEN="${PIPELINE_HAB_AUTH_TOKEN}"
export HAB_BLDR_URL="${PIPELINE_HAB_BLDR_URL}"

targets=("x86_64-linux"
         "x86_64-linux-kernel2"
         "x86_64-windows")

for target in "${targets[@]}"; do
    ident=$(worker_ident_for_target "${target}")
    echo "--- Promoting ${ident} (${target}) to '${promotion_channel}'"
    hab pkg promote \
        --auth="${HAB_AUTH_TOKEN}" \
        --url="${HAB_BLDR_URL}" \
        "${ident}" "${promotion_channel}" "${target}"
done
