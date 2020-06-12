#!/usr/bin/env bats

outfile="$BATS_TMPDIR/core-graph.dot"

load 'shared_setup'
load '../test_helper/bats-support/load'
load '../test_helper/bats-assert/load'
load '../test_helper/bats-file/load'

@test "Can generate a dot graph: serialized_db_connect $packages_db , dot $outfile core" {
    assert [ -f "$packages_db" ]
    run builder_graph -- serialized_db_connect "$packages_db" , dot "$outfile" core 
    # Assert the command ran successfully and generated the expected file
    assert_success
    assert_file_exist "$outfile"

    # The file should have the expected structure
    assert_file_contains "$outfile" "digraph \"$outfile\""
    assert_file_contains "$outfile" 'RUN TIME EDGES'
    assert_file_contains "$outfile" 'BUILD TIME EDGES'

    # Assert that a sample of known run and build edges are present
    assert_file_contains "$outfile" '"core/glibc" -> "core/linux-headers" \[type="R"\]'
    # gcc-libs version pins to the _same_ version of gcc. This test is slightly 
    # fragile in that if we ever update our serialized database, this test is
    # going to break. We want to ensure it points to the latest version and not 
    # just something that looks like a version. For now we accept that brittleness 
    # in order to provide some level of safety.
    assert_file_contains "$outfile" '"core/gcc-libs" -> "core/gcc/9.1.0" \[type="B"\]'
}

@test "Generated dot graph is a valid dot file" {
  if ! command -v dot >/dev/null; then
    skip "Unable to find dot command on the system"
  fi

  #  TODO: This tests depends on the previous test running first 
  #  and leaving state behind. This isn't ideal, but we need to 
  #  split this into its own test so it is skippable and don't want
  #  to pay the `serialized_db_connect` time tax
  run dot -O -Tpng "$outfile"
  assert_success
}
