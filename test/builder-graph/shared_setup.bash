builder_graph() {
    "${BLDR_GRAPH_PATH:-"$BATS_TEST_DIRNAME/../../target/debug/bldr-graph"}" "${@}"
}

# shellcheck disable=SC2034
packages_db="$BATS_TEST_DIRNAME/fixtures/db-core-2022-08-22.json"