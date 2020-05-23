#!/usr/bin/env bats

function setup() {
    source $BATS_TEST_DIRNAME/../lib/constants.bsc
}

@test "prints usage with -h, and exits 0" {
    run $BATS_TEST_DIRNAME/git-split -h
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"usage:"* ]]
}

@test "prints usage with --help, and exits 0" {
    run $BATS_TEST_DIRNAME/git-split --help
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"usage:"* ]]
}

@test "prints usage with help, and exits 0" {
    run $BATS_TEST_DIRNAME/git-split help
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"usage:"* ]]
}

@test "--version includes copyright notice" {
    run $BATS_TEST_DIRNAME/git-split --version
    echo "$output"
    [[ "$output" == *"Copyright"* ]]
    [[ "$output" == *"Affero"* ]]
    [[ "$output" == *"Nikita Skobov"* ]]
}

@test "--version up to date version number" {
    run $BATS_TEST_DIRNAME/git-split --version
    echo "$output"
    [[ "$output" == *"version $doc_version"* ]]
}
