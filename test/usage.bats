#!/usr/bin/env bats

@test "prints usage with -h, and exits 0" {
    run ./dist/git-split -h
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"Usage:"* ]]
}

@test "prints usage with --help, and exits 0" {
    run ./dist/git-split --help
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"Usage:"* ]]
}

@test "prints usage with help, and exits 0" {
    run ./dist/git-split help
    [[ "$status" -eq 0 ]]
    [[ "$output" = *"Usage:"* ]]
}
