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

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
    fi
    cd ..
    if [[ -d check ]]; then
        rm -rf check/
    fi
}

function setup() {
    test_folder="$BATS_TMPDIR/syncout"
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


@test 'sync out can push to a remote branch that user enters interactively' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    

    include=[\"abc.txt\", \"xyz.txt\"]
    "
    echo "$repo_file_contents" > repo_file.rf
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    # this is the local commit we have that can be pushed up to remote
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"

    # mgt sync is an interactive command, we put a list of our inputs into
    # a text file and feed that to its stdin.
    # this series of inputs should be:
    # 1. select push
    # <ENTER>. name of branch for remote to use
    expected_remote_branch="remotebranchhere"
    interact="1\n$expected_remote_branch\n"
    echo -e "$interact" > interact.txt

    # fork point should be calculated at abc commit, and then sync command
    # should report that we can push in updates to the remote repo
    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"You can push"* ]]
    [[ $output != *"You can pull"* ]]

    # no lingering branches, and we should still be on our original branch
    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]

    # now test that the remote repo received the expected branch
    cd "$BATS_TMPDIR/test_remote_repo2"
    remote_has_branches="$(git branch)"
    echo "Remote has branches now:"
    echo "$remote_has_branches"
    [[ "$remote_has_branches" == *"$expected_remote_branch"* ]]
}
