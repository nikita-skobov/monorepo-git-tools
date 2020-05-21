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

@test 'can detect if input file does not exist' {
    run ./dist/git-split_t out nonexistantfile.txt
    [[ "$status" -eq $ecf_failed_to_find_input_file ]]

    run ./dist/git-split_t in nonexistantfile.txt
    [[ "$status" -eq $ecf_failed_to_find_input_file ]]
}

@test 'can detect if failed to source' {
    echo "nonexistant_command_" > bad_source_file.txt

    run ./dist/git-split_t in bad_source_file.txt
    [[ "$status" -eq $ecf_source_failure ]]

    run ./dist/git-split_t out bad_source_file.txt
    [[ "$status" -eq $ecf_source_failure ]]

    rm bad_source_file.txt
}
