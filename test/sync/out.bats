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

# if we try to push to remote branch X, but
# X has diverged from what we think X is at, then
# git of course fails that push, but then
# we should properly recover, and leave the user
# back to a clean state where they started
@test 'sync out can recover from a failed push' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    git checkout -b newbranch
    echo "diverge" > xyz.txt && git add xyz.txt && git commit -m "xyz"
    git checkout -
    echo "REMOTE:"
    echo "$(git log --all --decorate --oneline --graph)"
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
    expected_remote_branch="newbranch"
    interact="1\n$expected_remote_branch\n"
    echo -e "$interact" > interact.txt

    # fork point should be calculated at abc commit, and then sync command
    # should report that we can push in updates to the remote repo
    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    # although it should have failed to push because
    # we cannot push to newbranch because it has diverged
    # it should still exit "success" because we did not specify --fail-fast
    # if we specified that, then it would exit an error as soon as one
    # of the sync items has an error
    [[ $status == "0" ]]
    [[ $output == *"You can push"* ]]
    [[ $output != *"You can pull"* ]]
    [[ $output == *"rejected"* ]] # git should reject the push

    # no lingering branches, and we should still be on our original branch
    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]
}

@test 'sync out --fail-fast will exit on first error it sees' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    git checkout -b newbranch
    echo "diverge" > xyz.txt && git add xyz.txt && git commit -m "xyz"
    git checkout -
    echo "REMOTE:"
    echo "$(git log --all --decorate --oneline --graph)"
    cd "$curr_dir"

    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    

    include=[\"abc.txt\", \"xyz.txt\"]
    "
    echo "$repo_file_contents" > repo_file.rf
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    

    include=[\"abc.txt\"]
    "
    echo "$repo_file_contents" > repo_file2.rf
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    # this is the local commit we have that can be pushed up to remote
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"

    # 1. select push
    # <ENTER>. name of branch for remote to use
    # for the second repo file:
    # 1. select push
    # <ENTER>, default branch name
    expected_remote_branch="newbranch"
    interact="1\n$expected_remote_branch\n1\n\n"
    echo -e "$interact" > interact.txt

    # Without --fail-fast, we should fail to sync the first repo file,
    # but the second one should succeed because the first error should be properly
    # cleaned up
    run $PROGRAM_PATH sync repo_file.rf repo_file2.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"You can push"* ]]
    [[ $output != *"You can pull"* ]]
    [[ $output == *"rejected"* ]] # git should reject the push
    # we should see up to date show up for the second repo file
    [[ $output == *"Up to date"* ]]

    # no lingering branches, and we should still be on our original branch
    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]

    # But now, we pass --fail-fast, so we should NOT see the second repo file be
    # synced, so we shouldnt see "Up to date"
    run $PROGRAM_PATH sync repo_file.rf repo_file2.rf --fail-fast --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status != "0" ]]
    [[ $output == *"You can push"* ]]
    [[ $output != *"You can pull"* ]]
    [[ $output == *"rejected"* ]] # git should reject the push
    # we should NOT see up to date show up for the second repo file
    [[ $output != *"Up to date"* ]]
}


@test 'sync out can stash index changes and then pop them afterwards' {
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
    # this is just a local change that should be stashed because
    # its not committed:
    echo "qqq" > abc.txt
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"

    # mgt detects unclean index, so it asks what we want to do:
    # 1. stash changes
    # 1. select push
    # <ENTER>. name of branch for remote to use
    expected_remote_branch="remotebranchhere"
    interact="1\n1\n$expected_remote_branch\n"
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

    cd "$curr_dir"
    # now test that our abc.txt file is back to qqq:
    abc_contents="$(cat abc.txt)"
    [[ "$abc_contents" == *"qqq"* ]]
}
