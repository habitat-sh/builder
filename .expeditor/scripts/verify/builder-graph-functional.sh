#!/bin/bash

set -euo pipefail

echo "--- Generating signing key"
hab origin key generate "$HAB_ORIGIN"

# TODO: core/graphviz doesn't appear to support png
# echo "--- Installing graphvis"
# hab pkg install core/graphviz 
# hab pkg binlink core/graphviz dot 

echo "--- Building builder-graph package"
hab pkg build components/builder-graph

source results/last_build.env
hab pkg install results/$pkg_artifact
echo "--- Running tests for $pkg_ident"
BLDR_GRAPH_PATH="$(hab pkg path "$HAB_ORIGIN"/builder-graph)/bin/bldr-graph"
export BLDR_GRAPH_PATH
echo "Using $BLDR_GRAPH_PATH"

test/builder-graph/test.sh