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
        SEP="\\\\"
    else
        SEP="/"
    fi
}

function setup() {
    test_folder="$BATS_TMPDIR/splitoutas"
    mkdir -p "$test_folder"
    BATS_TMPDIR="$test_folder"
    cd $test_folder
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
    cd ..
    if [[ -d splitoutas ]]; then
        rm -rf splitoutas/
    fi
}

@test 'requires an output branch name' {
    mkdir -p this
    mkdir -p this/path
    mkdir -p this/path/exists
    echo "file1" > this/path/exists/file1.txt
    git add this/ && git commit -m "file1"

    run $PROGRAM_PATH split-out-as --as this/path/exists
    echo "$output"
    [[ $status != "0" ]]
    [[ $output == *"Must provide an --output-branch"* ]]
}

@test 'moves everything in --as to the root of the newly created repo' {
    mkdir -p this
    mkdir -p this/path
    mkdir -p this/path/exists
    echo "file1" > this/path/exists/file1.txt
    git add this/ && git commit -m "file1"
    echo "rootfile1" > rootfile1.txt
    git add rootfile1.txt && git commit -m "rootfile1.txt"

    run $PROGRAM_PATH split-out-as --as this/path/exists/ -o newbranch -v
    echo "$output"
    echo "$(git branch -v)"
    echo "$(git log --oneline)"
    [[ "$(git branch --show-current)" == "newbranch" ]]
    [[ -f file1.txt ]]
    [[ ! -d this/ ]]
    [[ ! -f rootfile1.txt ]]
    [[ $status == "0" ]]
}
