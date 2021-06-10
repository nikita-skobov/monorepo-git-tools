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
    test_folder="$BATS_TMPDIR/sync"
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



@test 'should stay on starting branch and not leave any lingering branches after sync in' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
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
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"

    # mgt sync is an interactive command, we put a list of our inputs into
    # a text file and feed that to its stdin.
    # this series of inputs should be:
    # 1. select pull
    # 1. merge branch
    interact="1\n1\n"
    echo -e "$interact" > interact.txt

    # fork point should be calculated at abc commit, and then sync command
    # should report that we can pull in updates from the remote
    # repo.
    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"You can pull"* ]]
    [[ $output != *"You can push"* ]]

    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]
}

# like the above test, but now we pass 2. for the
# second option, so we should not merge, and instead
# remain on the temp branch
@test 'should stay on temp branch sync in' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
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
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"
    git_branch_before="$(git branch --show)"

    # mgt sync is an interactive command, we put a list of our inputs into
    # a text file and feed that to its stdin.
    # this series of inputs should be:
    # 1. select pull
    # 2. stay on temp branch
    interact="1\n2\n"
    echo -e "$interact" > interact.txt

    # fork point should be calculated at abc commit, and then sync command
    # should report that we can pull in updates from the remote
    # repo.
    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"You can pull"* ]]
    [[ $output != *"You can push"* ]]

    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    git_branch_after="$(git branch --show)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" != "$git_branches_after" ]]
    [[ "$git_branch_before" != "$git_branch_after" ]]
}


# like the above test, but now we pass 2. for the
# second option, so we should not merge, and instead
# remain on the temp branch
@test 'can specify a remote branch to fetch from with --ask-branches' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    # the topbase alg will only detect updates on the updateshere
    # branch, otherwise, the main branch will just be the fork point
    # so mgt sync would report no updates
    git checkout -b updateshere
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    git checkout -
    echo "REMOTE without updates:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    

    include=[\"abc.txt\", \"xyz.txt\"]
    "
    echo "$repo_file_contents" > repo_file.rf
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"

    # first we try what happens without ask branches
    # it should report there is nothing to update
    interact="1\n2\n"
    echo -e "$interact" > interact.txt

    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"Up to date"* ]]

    # but now, if we pass --ask-branches
    # and also provide the name of the branch in the interaction
    # then we should find the xyz commit to pull
    interact="updateshere\n1\n2\n"
    echo -e "$interact" > interact.txt

    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 --ask-branches < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output != *"Up to date"* ]]
    [[ $output == *"You can pull"* ]]
    [[ $output != *"You can push"* ]]
}


@test '--summary-only works for sync in' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
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
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"

    # this test should not have any interactive options
    # because we pass --summary-only, but just in case
    # we will fill the interaction file with non-valid
    # options that will cause an error:
    # 7. not valid
    interact="7\n7\n"
    echo -e "$interact" > interact.txt

    # fork point should be calculated at abc commit, and then sync command
    # should report that we can pull in updates from the remote
    # repo.
    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 --summary-only < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"You can pull"* ]]
    [[ $output != *"You can push"* ]]
    # even though it says "You can pull", it should
    # not actually give you the choice to perform a pull operation
    [[ $output != *"1. pull"* ]]

    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]
}

# the following two test cases are about stash/pop
# this first one checks if it works where we stash changes
# that are unrelated to what we sync in. This is a trivial case
# where we are pretty likely guaranteed that the sstash pop will succeed.
# the next test after this one will test if the sync in can stash pop
# when the stash is the same file (ie: conflict between the stash and what was
# pulled in).
@test 'sync in can stash the index changes and then pop afterwards (unrelated changes)' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
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
    # this should be stashed because although its added, its not
    # committed, so mgt should detect this as a dirty index
    echo "qqq" > abc.txt && git add abc.txt
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"
    # we of course should not have the xyz commit yet
    [[ "$(git log --oneline)" != *"xyz"* ]]

    # this series of inputs should be:
    # 1. stash changes
    # 1. select pull
    # 1. merge branch
    interact="1\n1\n1\n"
    echo -e "$interact" > interact.txt

    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"You can pull"* ]]
    [[ $output != *"You can push"* ]]

    # because we merged, we should have the xyz commit now
    [[ "$(git log --oneline)" == *"xyz"* ]]

    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]

    # now check the stash: we should have a qqq in our abc.txt file:
    abc_contents="$(cat abc.txt)"
    [[ "$abc_contents" == *"qqq"* ]]
}

@test 'sync in can stash the index changes and then pop afterwards (conflict changes)' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
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
    # this should be stashed because although its added, its not
    # committed, so mgt should detect this as a dirty index
    # this should also cause a conflict at the end
    # because it conflicts with the xyz.txt that we are pulling in
    echo "qqq" > xyz.txt && git add xyz.txt
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"
    # we of course should not have the xyz commit yet
    [[ "$(git log --oneline)" != *"xyz"* ]]

    # this series of inputs should be:
    # 1. stash changes
    # 1. select pull
    # 1. merge branch
    interact="1\n1\n1\n"
    echo -e "$interact" > interact.txt

    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status != "0" ]]
    [[ $output == *"You can pull"* ]]
    [[ $output != *"You can push"* ]]

    # because we merged, we should have the xyz commit now
    [[ "$(git log --oneline)" == *"xyz"* ]]

    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]

    echo "GIT STATUS NOW:"
    echo "$(git status)"

    # our git status should have a conflict:
    [[ "$(git status)" == *"both added"* ]]
}

@test 'should show up to date if the only commits to take are merge commits sync in' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    git checkout -b m1
    echo "xyz" > xyz.txt && git add xyz.txt && git commit -m "xyz"
    git checkout -
    echo "qqq" > qqq.txt && git add qqq.txt && git commit -m "qqq"
    git merge --no-ff m1

    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    

    include=[\"abc.txt\"]
    "
    echo "$repo_file_contents" > repo_file.rf
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"
    git_branches_before="$(git branch)"

    # it should not ask anything because it should show up to date
    interact=""
    echo -e "$interact" > interact.txt

    # fork point should be calculated at abc commit, and then sync command
    # should report that we can pull in updates from the remote
    # repo.
    run $PROGRAM_PATH sync repo_file.rf --max-interactive-attempts 1 < interact.txt
    echo "$output"
    [[ $status == "0" ]]
    [[ $output != *"You can pull"* ]]
    [[ $output != *"You can push"* ]]
    [[ $output == *"Up to date"* ]]

    echo "Git branches before:"
    echo "$git_branches_before"
    git_branches_after="$(git branch)"
    echo "Git branches after:"
    echo "$git_branches_after"
    [[ "$git_branches_before" == "$git_branches_after" ]]
}
