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
