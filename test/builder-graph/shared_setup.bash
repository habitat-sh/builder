builder_graph() {
    "${BLDR_GRAPH_PATH:-"$BATS_TEST_DIRNAME/../../target/debug/bldr-graph"}" "${@}"
}

packages_db="$BATS_TEST_DIRNAME/fixtures/db-core-2020-05-18.json"