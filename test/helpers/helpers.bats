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

@test 'get_remote_repo_from_args can detect valid git urls' {
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

    run get_remote_repo_from_args "https//host.net/repo/path.git"
    [[ $status -eq 1 ]]
}

@test 'get_remote_repo_from_args sets remote_repo variable' {
    get_remote_repo_from_args "ssh://user@host.net/repo/path1"
    [[ $remote_repo == "ssh://user@host.net/repo/path1" ]]
    get_remote_repo_from_args "https://host.net/repo/path7/"
    [[ $remote_repo == "https://host.net/repo/path7/" ]]
}

@test 'get_repo_name_from_remote_repo sets repo_name correctly' {
    get_repo_name_from_remote_repo "ssh://user@host.net/repo/path1"
    [[ $repo_name == "path1" ]]
    get_repo_name_from_remote_repo "git://host.net/repo/path2"
    [[ $repo_name == "path2" ]]
    get_repo_name_from_remote_repo "https://host.net/repo/path3"
    [[ $repo_name == "path3" ]]
    get_repo_name_from_remote_repo "ftp://host.net:port/repo/path4"
    [[ $repo_name == "path4" ]]
    get_repo_name_from_remote_repo "user@host.net:/repo/path5"
    [[ $repo_name == "path5" ]]
    get_repo_name_from_remote_repo "https://host.net/repo/path6.git"
    [[ $repo_name == "path6" ]]
    get_repo_name_from_remote_repo "https://host.net/repo/path7/"
    [[ $repo_name == "path7" ]]
}

@test 'get_log_no_merges_with_format works for hashes' {
    run get_log_no_merges_with_format "master" "%h"
    for hash in $output; do
        hash_length="${#hash}"
        [[ $hash_length == 8 ]]
        break
    done
}

@test 'get_log_no_merges_with_format works for author dates' {
    run get_log_no_merges_with_format "master" "%at"
    for author_date in $output; do
        # check if is a number:
        [[ $author_date =~ ^[0-9]+$ ]]
        break
    done
}
