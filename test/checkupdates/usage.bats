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

function set_seperator() {
    # I wanna use these tests for both windows (git bash)
    # and linux, so I need to change the separator
    if [[ -d /c/ ]]; then
        SEP="\\"
    else
        SEP="/"
    fi
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

function setup() {
    set_seperator
    make_temp_repo test_remote_repo
    test_remote_repo="test_remote_repo"
    make_temp_repo test_remote_repo2
    test_remote_repo2="test_remote_repo2"
    cd $BATS_TMPDIR/test_remote_repo
}



@test 'should not allow both --local and --remote at same time' {
    run $PROGRAM_PATH check-updates --local --remote repo_file.txt
    echo "$output"
    [[ $status != "0" ]]
    [[ "$output" == *"cannot be used with"* ]]
}

@test 'can optionally specify a remote branch to override repo file' {
    # by default it should be whatever is in the repo file branch:
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    remote_branch=\"somebranch\"
    "

    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH check-updates repo_file.sh
    echo "$output"
    [[ "$output" == *"Upstream: HEAD"* ]]
    [[ "$output" == *"Current: ..$SEP$test_remote_repo2 somebranch"* ]]

    # if we specify --remote otherbranch, it should override the default
    run $PROGRAM_PATH check-updates repo_file.sh --remote -b other
    echo "$output"
    [[ "$output" == *"Upstream: HEAD"* ]]
    [[ "$output" == *"Current: ..$SEP$test_remote_repo2 other"* ]]
}

@test 'can optionally specify a local branch to check from/to' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    "

    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH check-updates repo_file.sh --local
    echo "$output"
    [[ "$output" == *"Current: HEAD"* ]]
    [[ "$output" == *"Upstream: ..$SEP$test_remote_repo2"* ]]

    run $PROGRAM_PATH check-updates repo_file.sh --local --local-branch other
    echo "$output"
    [[ "$output" == *"Current: other"* ]]
    [[ "$output" == *"Upstream: ..$SEP$test_remote_repo2"* ]]
}


@test 'uses remote_repo:HEAD by default' {
    # by default it should be whatever is in the repo file branch:
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    "

    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH check-updates repo_file.sh
    echo "$output"
    [[ "$output" == *"Upstream: HEAD"* ]]
    [[ "$output" == *"Current: ..$SEP$test_remote_repo2"* ]]
}
