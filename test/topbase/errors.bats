function make_temp_repo() {
    cd $BATS_TMPDIR
    mkdir -p $1
    cd $1
    if [[ ! -d .git ]]; then
        git init
        git config --local user.email "temp"
        git config --local user.name "temp"
        echo "name of repo: $1" > $1.txt
        git add $1.txt
        git commit -m "initial commit for $1"
    fi
}

function setup() {
    source $BATS_TEST_DIRNAME/../../lib/constants/exit_codes.bsc
    source $BATS_TEST_DIRNAME/../../lib/helpers.bsc
    make_temp_repo test_remote_repo
    make_temp_repo test_remote_repo2
    cd $BATS_TMPDIR/test_remote_repo
}

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
    fi
}

@test 'checks if both top and bottom branches exist' {
    run $BATS_TEST_DIRNAME/git-topbase bad_branch1 bad_branch2
    [[ $output == *"Failed to find branch: bad_branch1"* ]]
    [[ $status -eq $ecf_invalid_branch ]]

    run $BATS_TEST_DIRNAME/git-topbase master bad_branch2
    [[ $output == *"Failed to find branch: bad_branch2"* ]]
    [[ $status -eq $ecf_invalid_branch ]]
}
