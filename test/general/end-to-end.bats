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

function setup() {
    set_seperator
    make_temp_repo test_remote_repo
    test_remote_repo="test_remote_repo"
    make_temp_repo test_remote_repo2
    test_remote_repo2="test_remote_repo2"
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

@test 'can use same repo_file for split-in and split-out' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \" \"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    echo "$output"
    [[ $status == "0" ]]

    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]
    [[ ! -f test_remote_repo2.txt ]]

    # now we test if it works in reverse
    run $PROGRAM_PATH split-out repo_file.sh --verbose -o original
    [[ $status == "0" ]]
    #echo "$(find . -not -path '*/\.*')"
    [[ "$(git branch --show-current)" == *"original"* ]]
    [[ -f test_remote_repo2.txt ]]
    [[ ! -f this/path/will/be/created/test_remote_repo2.txt ]]
    [[ ! -d this ]]
}

@test 'split-in by default should fail if the output branch it wants to create already exists' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \" \"
    )
    "

    # make sure it doesnt exist first
    [[ "$(git branch)" != *"test_remote_repo2"* ]]

    echo "$repo_file_contents" > repo_file.sh
    run $PROGRAM_PATH split-in repo_file.sh --verbose
    # this should be fine because test_remote_repo2 doesnt exist yet
    echo "$output"
    [[ $status == "0" ]]

    # ensure it was created
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]

    # now if we run it again,
    # it should fail because that branch already exists
    git checkout master
    run $PROGRAM_PATH split-in repo_file.sh --verbose
    echo "$output"
    [[ "$status" != "0" ]]
    [[ "$output" == *"Failed to checkout orphan branch"* ]]
}

@test 'split-out by default should fail if the output branch it wants to create already exists' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"test_remote_repo.txt\"
    "

    # make sure it doesnt exist first
    [[ "$(git branch)" != *"test_remote_repo2"* ]]

    echo "$repo_file_contents" > repo_file.sh
    run $PROGRAM_PATH split-out repo_file.sh --verbose
    # this should be fine because test_remote_repo2 doesnt exist yet
    echo "$output"
    [[ $status == "0" ]]

    # ensure it was created
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]

    # now if we run it again,
    # it should fail because that branch already exists
    git checkout master
    run $PROGRAM_PATH split-out repo_file.sh --verbose
    echo "$output"
    [[ "$status" != "0" ]]
    [[ "$output" == *"Failed to checkout"* ]]
}
