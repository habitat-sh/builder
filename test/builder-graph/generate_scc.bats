#!/usr/bin/env bats

load 'shared_setup'
load '../test_helper/bats-support/load'
load '../test_helper/bats-assert/load'

@test "Can generate a scc file" {
    outfile=/tmp/core.scc
    run builder_graph -- serialized_db_connect "$packages_db" , scc "$outfile" core
    assert [ "$status" -eq 0 ]
    assert [ -f "$outfile" ]
}