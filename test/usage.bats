#!/usr/bin/env bats

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
