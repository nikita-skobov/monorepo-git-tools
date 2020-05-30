#!/usr/bin/env bats

function setup() {
    source ./lib/constants/exit_codes.bsc
}


@test 'can detect missing input' {
    run $BATS_TEST_DIRNAME/git-split
    [[ "$status" -eq $ecf_missing_input ]]

    run $BATS_TEST_DIRNAME/git-split out
    [[ "$status" -eq $ecf_missing_input ]]

    run $BATS_TEST_DIRNAME/git-split in
    [[ "$status" -eq $ecf_missing_input ]]
}

@test 'can detect if failed to source' {
    echo "nonexistant_command_" > bad_source_file.txt

    run $BATS_TEST_DIRNAME/git-split in bad_source_file.txt
    [[ "$status" -eq $ecf_source_failure ]]

    run $BATS_TEST_DIRNAME/git-split out bad_source_file.txt
    [[ "$status" -eq $ecf_source_failure ]]

    rm bad_source_file.txt
}

@test 'can detect if input is an invalid repo uri or nonexistant file' {
    run $BATS_TEST_DIRNAME/git-split in http:/badrepo.uri/path
    [[ "$status" -eq $ecf_invalid_repo_uri ]]

    run $BATS_TEST_DIRNAME/git-split out ssh:/badhost/path
    [[ "$status" -eq $ecf_invalid_repo_uri ]]

    run $BATS_TEST_DIRNAME/git-split out nonexistant.txt
    [[ "$status" -eq $ecf_invalid_repo_uri ]]
}
