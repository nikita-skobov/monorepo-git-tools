#!/usr/bin/env bats

function setup() {
    source ./lib/constants.bsc
}


@test 'can detect missing input' {
    run $BATS_TEST_DIRNAME/git-split
    [[ "$status" -eq $ecf_missing_input ]]

    run $BATS_TEST_DIRNAME/git-split out
    [[ "$status" -eq $ecf_missing_input ]]

    run $BATS_TEST_DIRNAME/git-split in
    [[ "$status" -eq $ecf_missing_input ]]
}

@test 'can detect if input file does not exist' {
    run $BATS_TEST_DIRNAME/git-split out nonexistantfile.txt
    [[ "$status" -eq $ecf_failed_to_find_input_file ]]

    run $BATS_TEST_DIRNAME/git-split in nonexistantfile.txt
    [[ "$status" -eq $ecf_failed_to_find_input_file ]]
}

@test 'can detect if failed to source' {
    echo "nonexistant_command_" > bad_source_file.txt

    run $BATS_TEST_DIRNAME/git-split in bad_source_file.txt
    [[ "$status" -eq $ecf_source_failure ]]

    run $BATS_TEST_DIRNAME/git-split out bad_source_file.txt
    [[ "$status" -eq $ecf_source_failure ]]

    rm bad_source_file.txt
}
