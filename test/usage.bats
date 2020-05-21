#!/usr/bin/env bats

@test "prints usage with -h, and exits 0" {
    run ./dist/git-split_t -h
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"Usage:"* ]]
}

@test "prints usage with --help, and exits 0" {
    run ./dist/git-split_t --help
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"Usage:"* ]]
}

@test "prints usage with help, and exits 0" {
    run ./dist/git-split_t help
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"Usage:"* ]]
}
