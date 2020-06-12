#!/usr/bin/env bats

outfile="$BATS_TMPDIR/core_build_order.txt"

load 'shared_setup'
load '../test_helper/bats-support/load'
load '../test_helper/bats-assert/load'

@test "Can generate a build order: serialized_db_connect $packages_db , build_order $outfile core core/gcc" {
    assert [ -f "$packages_db" ]
    run builder_graph -- serialized_db_connect "$packages_db" , build_order "$outfile" core core/gcc
    assert [ "$status" -eq 0 ]
    assert_output --partial "Generated build order"
    assert [ -f "$outfile" ]

    # things not in cycles only appear once
    assert [ "$(pcregrep -c '^core/wordpress\s' "$outfile")" -eq 1 ]
    # things in cycles appear 3 times
    assert [ "$(pcregrep -c '^core/gcc\s' "$outfile")" -eq 3 ]
    assert [ "$(pcregrep -c '^core/ghc\s' "$outfile")" -eq 3 ]

    # gcc should happen before gcc-libs 
    first_gcc="$(pcregrep -n -o0 '^core/gcc\s' "$outfile" | cut -d: -f1 | head -1)"
    first_gcc_libs="$(pcregrep -n -o0 '^core/gcc-libs\s' "$outfile" | cut -d: -f1 | head -1)"
    assert [ "$first_gcc" -lt "$first_gcc_libs" ]

    # wordpress should happen after the last gcc
    last_gcc="$(pcregrep -n -o0 '^core/gcc\s' "$outfile" | cut -d: -f1 | tail -1)"
    wordpress="$(pcregrep -n -o0 '^core/wordpress\s' "$outfile" | cut -d: -f1 | head -1)"
    assert [ "$last_gcc" -lt "$wordpress" ]

}