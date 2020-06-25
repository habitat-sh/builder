#!/bin/bash

set -euo pipefail

project_root="$(git rev-parse --show-toplevel)"

function install_bats_library() {
    local library="${1:?Specify library name}"
    local library_install_path="$project_root/test/test_helper/$library"
    local bats_core_github="https://github.com/bats-core/"

    test -d "$library_install_path" || \
      git clone "$bats_core_github/$library" "$library_install_path"
}

( 
    cd "$project_root"
    install_bats_library "bats-support"
    install_bats_library "bats-assert"
    install_bats_library "bats-file"

    bats "${@:-test/builder-graph}"
)