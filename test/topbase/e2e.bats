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
        git commit -m "added extra line to $1"
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
