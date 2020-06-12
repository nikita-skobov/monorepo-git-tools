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

function make_commit() {
    if [[ -d .git ]]; then
        echo "extra line" >> $1
        git add $1
        if [[ ! -z $2 ]]; then
            git commit -m "commit $2"
        else
            git commit -m "added extra line to $1"
        fi
    fi
}

function get_last_n_commits() {
    git log --pretty=format:%h -n $1
}

function setup() {
    make_temp_repo tr1
    # make_temp_repo tr2
    cd $BATS_TMPDIR/tr1
}

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d tr1 ]]; then
        rm -rf tr1
    fi
    if [[ -d tr2 ]]; then
        rm -rf tr2
    fi
}

@test "will use the same commit hashes if a simple FF applies" {
    git checkout -b new_branch
    number_commits_to_make=8
    for ((j = 0; j < number_commits_to_make; j += 1)); do
        make_commit "tr1.txt"
    done

    # get the last n commits from newbranch
    # and go back to master to verify those werent applied to master
    new_branch_latest_commits="$(get_last_n_commits $number_commits_to_make)"
    git checkout master
    current_master_commits="$(git log --oneline | wc -l)"
    [[ $current_master_commits == 1 ]]

    # do the topbase:
    $BATS_TEST_DIRNAME/git-topbase new_branch master

    # verify that master has n+1 commits
    current_master_commits="$(git log --oneline | wc -l)"
    [[ $current_master_commits == "$((number_commits_to_make+1))" ]]

    # verify that the new n commits have the same hashes as what it
    # would have been if you FF-ed.
    master_branch_latest_commits="$(get_last_n_commits $number_commits_to_make)"
    [[ $new_branch_latest_commits == $master_branch_latest_commits ]]
}


@test "will skip merge commits" {
    git checkout -b new_branch
    number_commits_to_make=4
    for ((j = 0; j < number_commits_to_make; j += 1)); do
        make_commit "tr1.txt" $j
    done

    git checkout -b tmp1
    make_commit "tr2.txt" "$j"

    # force a merge commit from tmp1 into new_branch
    git checkout new_branch
    git merge --no-edit --no-ff tmp1
    git branch -D tmp1

    # make a commit on top of the merge commit
    ((j+=1))
    make_commit "tr3.txt" "$j"

    # at this point new_branch has
    # a merge commit, and then another commit on top
    # first check to make sure it has a merge commit
    run git log --oneline
    [[ $output == *"Merge branch 'tmp1' into new_branch"* ]]

    $BATS_TEST_DIRNAME/git-topbase new_branch master

    # then, after topbasing, newbranch
    # should NOT have that merge commit
    run git log --oneline
    [[ $output != *"Merge branch 'tmp1' into new_branch"* ]]
}

@test "running output of --dry-run yields same result as running without --dry-run" {
    git checkout -b new_branch
    number_commits_to_make=4
    for ((j = 0; j < number_commits_to_make; j += 1)); do
        make_commit "tr1.txt" $j
    done
    # get the last n commits from newbranch
    new_branch_latest_commits="$(get_last_n_commits $number_commits_to_make)"

    # verify those werent applied to master
    current_master_commits="$(git log master --oneline | wc -l)"
    [[ $current_master_commits == 1 ]]

    git checkout master
    make_commit "tr2.txt" "$j"
    # master now has 2 commits
    master_commits=2

    # this branch will receive the
    # eval of the dry-run
    git branch new_branch_dry_run_eval new_branch
    git checkout new_branch_dry_run_eval

    # do the dry run:
    run $BATS_TEST_DIRNAME/git-topbase new_branch_dry_run_eval master --dry-run
    eval "$output"

    # verify that the commits were applied onto new_branch_dry_run_eval:
    current_branch_commits="$(git log --oneline | wc -l)"
    dry_run_log="$(git log --oneline)"
    [[ $current_branch_commits == "$((number_commits_to_make + master_commits))" ]]

    # go back to new_branch and run without --dry-run
    git checkout new_branch

    # verify that new_branch was not changed:
    current_branch_commits="$(git log --oneline | wc -l)"
    [[ $current_branch_commits != "$((number_commits_to_make + master_commits))" ]]

    # run it regularly:
    $BATS_TEST_DIRNAME/git-topbase new_branch master

    # verify that the commits were applied:
    current_branch_commits="$(git log --oneline | wc -l)"
    [[ $current_branch_commits == "$((number_commits_to_make + master_commits))" ]]

    # verify that the logs match:
    regular_run_log="$(git log --oneline)"
    [[ $regular_run_log == $dry_run_log ]]

    # verify master still has $master_commits commits
    master_has_commits="$(git log master --oneline | wc -l)"
    [[ $master_has_commits == $master_commits ]]
}
