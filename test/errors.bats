#!/usr/bin/env bats

function setup() {
    source ./lib/constants.bsc
}


@test 'can detect missing input' {
    run ./dist/git-split_t
    [[ "$status" -eq $ecf_missing_input ]]

    run ./dist/git-split_t out
    [[ "$status" -eq $ecf_missing_input ]]

    run ./dist/git-split_t in
    [[ "$status" -eq $ecf_missing_input ]]
}
