#!/bin/bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

cargo clippy --fix \
  --allow-dirty \
  -p habitat_builder_api \
  --all-targets \
  --tests
