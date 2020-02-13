#!/bin/bash

# Download public and private keys for the given origin from Builder.
import_keys() {
    local origin="${1}"

    echo "--- :key: Downloading '${origin}' public keys from ${HAB_BLDR_URL}"
    hab origin key download "${origin}"
    echo "--- :closed_lock_with_key: Downloading latest '${origin}' secret key from ${HAB_BLDR_URL}"
    hab origin key download \
        --auth="${HAB_AUTH_TOKEN}" \
        --secret \
        "${origin}"
}

curlbash_hab() {
    local pkg_target="${1:-$BUILD_PKG_TARGET}"
    echo "--- :habicat: Installing current stable hab binary for $pkg_target using curl|bash"
    curl https://raw.githubusercontent.com/habitat-sh/habitat/master/components/hab/install.sh | sudo bash -s -- -t "$pkg_target"
}

set_worker_ident_for_target() {
    package_ident="${1}"
    target="${2}"

    echo "--- Registering ${package_ident} (${target})"
    buildkite-agent meta-data set "${target}-builder-worker" "${package_ident}"
}

worker_ident_for_target() {
    target="${1}"
    buildkite-agent meta-data get "${target}-builder-worker"
}
