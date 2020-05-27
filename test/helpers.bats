#!/usr/bin/env bats

function setup() {
    source ./lib/helpers.bsc
}

# not going to test all helper functions
# just the ones that are imperative they give the correct output:

@test 'is_array works' {
    my_array=("hello" "world")
    run is_array my_array
    [[ "$status" -eq 0 ]]

    my_string="string with spaces"
    run is_array my_string
    [[ "$status" -eq 1 ]]
}

@test 'branch_exists works' {
    current_branch=$(git rev-parse --abbrev-ref HEAD)
    # just going to assume this branch name doesnt exist:
    git checkout -b test-test-test-xyz-test
    run branch_exists test-test-test-xyz-test
    [[ "$status" -eq 0 ]]

    git checkout "$current_branch"
    git branch -D test-test-test-xyz-test
    run branch_exists test-test-test-xyz-test
    [[ "$status" -eq 1 ]]
}

@test 'get_remote_repo_from_args returns true for valid git urls' {
    run get_remote_repo_from_args "ssh://user@host.net/repo/path"
    [[ $status -eq 0 ]]
    run get_remote_repo_from_args "git://host.net/repo/path"
    [[ $status -eq 0 ]]
    run get_remote_repo_from_args "https://host.net/repo/path"
    [[ $status -eq 0 ]]
    run get_remote_repo_from_args "ftp://host.net:port/repo/path"
    [[ $status -eq 0 ]]
    run get_remote_repo_from_args "user@host.net:/repo/path"
    [[ $status -eq 0 ]]
    run get_remote_repo_from_args "https://host.net/repo/path.git"
    [[ $status -eq 0 ]]
}

@test 'get_remote_repo_from_args sets remote_repo variable' {
    get_remote_repo_from_args "ssh://user@host.net/repo/path1"
    [[ $remote_repo == "path1" ]]
    get_remote_repo_from_args "git://host.net/repo/path2"
    [[ $remote_repo == "path2" ]]
    get_remote_repo_from_args "https://host.net/repo/path3"
    [[ $remote_repo == "path3" ]]
    get_remote_repo_from_args "ftp://host.net:port/repo/path4"
    [[ $remote_repo == "path4" ]]
    get_remote_repo_from_args "user@host.net:/repo/path5"
    [[ $remote_repo == "path5" ]]
    get_remote_repo_from_args "https://host.net/repo/path6.git"
    [[ $remote_repo == "path6" ]]
    get_remote_repo_from_args "https://host.net/repo/path7/"
    [[ $remote_repo == "path7" ]]
}