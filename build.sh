#!/bin/bash

set -eou pipefail

component=${1?component argument required}

source support/ci/shared_build_environment.sh

cd "components/$component"
cargo build
