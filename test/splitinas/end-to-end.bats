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

@test 'can split-in-as a remote uri without a repo_file' {
    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $PROGRAM_PATH split-in-as "$BATS_TMPDIR/test_remote_repo2" --as this/path/will/be/created/ --verbose
    echo "$output"
    echo "$(find . -not -path '*/\.*')"
    [[ $status == "0" ]]

    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]
    [[ ! -f test_remote_repo2.txt ]]
}
