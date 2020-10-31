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
    test_folder="$BATS_TMPDIR/splitinas"
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

function make_commit() {
    echo "$1" > "$1"
    git add "$1"
    git commit -m "$1" > /dev/null
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
    if [[ -d splitinas ]]; then
        rm -rf splitinas/
    fi
}

@test 'can split-in-as a remote uri without a repo_file' {
    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $PROGRAM_PATH split-in-as "..$SEP$test_remote_repo2" --as this/path/will/be/created/ --verbose
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

# rebase onto means the changes stay on the new branch, but
# it uses the original branch as the upstream branch to compare with
@test 'can optionally rebase the new splitinas branch onto original branch' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    master_commits="$(git log --oneline | wc -l)"
    made_commits=1 # start at 1 because it has an initial commit
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    ((made_commits += 1))
    cd "$curr_dir"

    run $PROGRAM_PATH split-in-as "..$SEP$test_remote_repo2" --as abc/ --verbose --rebase
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    output_commits="$(git log --oneline | wc -l)"
    echo ""
    echo "$(git log --oneline)"
    echo ""

    # we test that the number of commits is now the number that we made in master
    # plus the number we made in the new branch that got filtered.
    echo "output_commits ($output_commits) =?= master_commits + made_commits ($((master_commits + made_commits)))"
    [[ "$output_commits" == "$((master_commits + made_commits))" ]]

    [[ -f abc/lib/libfile1.txt ]]
    [[ -f abc/lib/libfile2.txt ]]
    # since we specified to rebase, this file should exist
    # because it existed in master, and we rebased our new branch on top of master
    [[ -f test_remote_repo.txt ]]
}

# ie:
# git checkout -b B
# # make some commits
# git checkout -b B-rebased-on-master
# git rebase master
# git checkout master
# git merge B-rebased-on-master
# git checkout B
# git rebase master <- git CLI handles this just fine. B will be exactly the same as master in this case
@test 'rebasing the new splitinas branch onto original branch does nothing if original branch already has latest changes' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    master_commits="$(git log --oneline | wc -l)"
    made_commits=1 # start at 1 because it has an initial commit
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    ((made_commits += 1))
    cd "$curr_dir"

    run $PROGRAM_PATH split-in-as "..$SEP$test_remote_repo2" --as abc/ --verbose --rebase
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    output_commits="$(git log --oneline | wc -l)"
    echo ""
    echo "$(git log --oneline)"
    echo ""

    # we test that the number of commits is now the number that we made in master
    # plus the number we made in the new branch that got filtered.
    echo "output_commits ($output_commits) =?= master_commits + made_commits ($((master_commits + made_commits)))"
    [[ "$output_commits" == "$((master_commits + made_commits))" ]]

    [[ -f abc/lib/libfile1.txt ]]
    [[ -f abc/lib/libfile2.txt ]]
    # since we specified to rebase, this file should exist
    # because it existed in master, and we rebased our new branch on top of master
    [[ -f test_remote_repo.txt ]]

    echo "$(git branch)"

    # now we run it again and ensure that it didn't make any extra commits than last time
    # since theres no new history, it should be the same commit
    git checkout master
    git merge test_remote_repo2
    git branch -D test_remote_repo2
    latest_commit="$(git log --oneline -n 1)"
    
    run $PROGRAM_PATH split-in-as "..$SEP$test_remote_repo2" --as abc/ --verbose --rebase
    echo "$output"
    new_latest_commit="$(git log --oneline -n 1)"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    [[ "$new_latest_commit" == "$latest_commit" ]]
}

# topbasing is basically rebasing but calculating the forking point
# ourselves instead of trusting git to do it. git sometimes struggles
# with this when there are squash commits
@test 'can get latest changes using \"topbase\"' {
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    [include_as]
    \"lib/\" = \" \"
    "
    echo "$repo_file_contents" > repo_file.sh
    # the repo_file wont be committed

    # save current dir to cd back to later
    curr_dir="$PWD"
    make_commit A
    make_commit B
    make_commit C
    echo -e "\nlocal $(git branch --show-current) :"
    echo "$(git log --oneline)"

    # setup the remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    make_commit H
    make_commit I
    make_commit J
    make_commit K
    make_commit L
    echo -e "\nremote $(git branch --show-current) :"
    echo "$(git log --oneline)"

    # split in the remote repo into our local repo and squash merge
    cd "$curr_dir"
    run $PROGRAM_PATH split-in repo_file.sh --rebase -o outbranch
    git checkout master > /dev/null
    git merge --squash outbranch > /dev/null
    git commit -m "X" > /dev/null
    git branch -D outbranch > /dev/null
    echo -e "\nlocal $(git branch --show-current) :"
    echo "$(git log --oneline)"

    # make modifications to their code
    echo -e "\n\nHE" >> lib/H
    echo -e "\n\nIF" >> lib/I
    git add lib/H && git commit -m "E" > /dev/null
    git add lib/I && git commit -m "F" > /dev/null

    # split out and 'contribute' back to remote
    run $PROGRAM_PATH split-out repo_file.sh > /dev/null
    echo -e "\nlocal $(git branch --show-current) :"
    echo "$(git log --oneline)"
    cd "$BATS_TMPDIR/test_remote_repo2"
    git checkout --orphan from_local > /dev/null 2>&1
    git rm -rf . > /dev/null 2>&1
    git pull "$curr_dir" > /dev/null 2>&1
    git rebase master > /dev/null 2>&1
    git checkout master > /dev/null 2>&1
    git merge from_local --no-ff > /dev/null 2>&1
    git branch -D from_local > /dev/null 2>&1

    # they make modifications on top of your changes
    echo -e "\n\nHM" >> H
    echo -e "\n\nIN" >> I
    echo -e "\n\nKO" >> K
    git add H && git commit -m "M" > /dev/null
    git add I && git commit -m "N" > /dev/null
    git add K && git commit -m "O" > /dev/null
    echo -e "\nremote $(git branch --show-current) :"
    echo "$(git log --oneline)"

    # meanwhile you also made modifications on top of your changes
    cd "$curr_dir"
    git checkout master
    echo -e "\n\nAG" >> lib/A
    git add lib/A && git commit -m "G" > /dev/null
    git_log_master_before_topbase="$(git log --oneline)"
    echo -e "\nlocal $(git branch --show-current) :"
    echo "$(git log --oneline)"

    # now we split in and try to update from the most recent remote changes
    # we want the remote changes to go on top of whatever we have on master
    git branch -D test_remote_repo2
    run $PROGRAM_PATH split-in repo_file.sh
    echo -e "\nlocal $(git branch --show-current) :"
    echo "$(git log --oneline)"

    # a git rebase should fail because it fails to account
    # for the squashed initial pull
    run git rebase master
    [[ $status != "0" ]]
    [[ $output == *"error: could not apply"* ]]

    # get out of failed state
    git rebase --abort

    # now lets go back to master, and try to split in again, but this
    # time we will topbase
    git checkout master > /dev/null
    git branch -D test_remote_repo2

    # now a topbase should work
    run $PROGRAM_PATH split-in repo_file.sh -t --verbose
    echo "$output"
    [[ $status == "0" ]]
    git_log_now="$(git log --oneline)"
    echo -e "\nlocal $(git branch --show-current) :"
    echo "$(git log --oneline)"

    # if topbase works, it should put everything on top of
    # what master already had, so we check that it still has everything that
    # was in master before:
    [[ "$git_log_now" == *"$git_log_master_before_topbase"* ]]

    # and we also check if it has M, N, O now:
    [[ "$git_log_now" == *" M"* ]]
    [[ "$git_log_now" == *" N"* ]]
    [[ "$git_log_now" == *" O"* ]]
}

@test 'can generate a repo file' {
    run $PROGRAM_PATH split-in-as "..$SEP$test_remote_repo2" --as abc/ --gen-repo-file --verbose
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    [[ -f test_remote_repo2.rf ]]
    meta_rf_contents=$(<test_remote_repo2.rf)
    [[ "$meta_rf_contents" == *"[repo]"* ]]
    [[ "$meta_rf_contents" == *"remote = \"..$SEP$test_remote_repo2\""* ]]
    [[ "$meta_rf_contents" == *"[include_as]"* ]]
    [[ "$meta_rf_contents" == *"\"abc/\" = \" \""* ]]
}
