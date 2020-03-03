#!/usr/bin/env bats

outfile=$BATS_TMPDIR/core_ruby_rdeps.txt

load 'shared_setup'
load '../test_helper/bats-support/load'
load '../test_helper/bats-assert/load'
load '../test_helper/bats-file/load'

@test "Can generate rdeps: serialized_db_connect $packages_db , rdeps $outfile core/ruby" {
    assert [ -f $packages_db ]
    run builder_graph -- serialized_db_connect $packages_db , rdeps $outfile core/ruby core 
    assert [ "$status" -eq 0 ]
    assert [ -f $outfile ]

    # We should not list core/ruby as a dependency of itself, but it will exist on the first line
    # of the file
    assert [ $(pcregrep -c '^core/ruby$' $outfile) -eq 1 ]

    # Known dependencies that should change infrequently
    assert_file_contains "$outfile" "core/dd-agent"
    assert_file_contains "$outfile" "core/sentinel"
    assert_file_contains "$outfile" "core/scaffolding-ruby"
    assert_file_contains "$outfile" "core/fluentd"
    assert_file_contains "$outfile" "core/clojure"

    # Things that should not be dependencies
    assert [ $(pcregrep -c 'core/gcc' $outfile) -eq 0 ]
    assert [ $(pcregrep -c 'core/corretto' $outfile) -eq 0 ]
}