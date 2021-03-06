function setup() {
    echo "$PWD"
    # unfortunately this number needs to
    # be updated every time you add a test...
    LAST_TEST_NUMBER=1
    ORIGINAL_DIR="$PWD"
    test_folder="$BATS_TMPDIR/gfr-compat"


    # if [[ ! -d "$test_folder" ]]; then
        mkdir -p "$test_folder"
        cd $test_folder
        git init
        git config --local user.name "test"
        git config --local user.email "test"
    # fi

    BATS_TMPDIR="$test_folder"
    cd $BATS_TMPDIR
}

function teardown() {
    echo "$PWD"
    # if the filtered.txt is left over after
    # the test, output it, so its visible in the error log
    cd $BATS_TMPDIR
    if [[ -f filtered.txt ]]; then
        echo "YOURS: =================="
        cat filtered.txt
        # ls -la .git/filter-repo
        if [[ -f .git/filter-repo/fast-export.filtered ]]; then
            echo "THEIRS:=================="
            cat .git/filter-repo/fast-export.filtered
        fi
    fi
    cd ..
    # if [[ "$BATS_TEST_NUMBER" == "$LAST_TEST_NUMBER" ]]; then
        echo "Test done, removing temporary directory"
        # comment this out if you want to run the test just
        # once and then examine the files produced.
        if [[ -d gfr-compat ]]; then
            rm -rf gfr-compat/
        fi
    # fi
}

# get the git commit hash for the commit
# at the top of this branch, defaults to HEAD
# if $1 not provided
function hash_atop() {
    thing="$1"
    refname="${thing:=HEAD}"
    git rev-parse --verify "$refname"
}

# get number of commits for the branch in $1
# or defaults to HEAD
function num_commits() {
    thing="$1"
    refname="${thing:=HEAD}"
    git log --oneline "$refname" | wc -l
}

function make_file() {
    echo "$1" > "$1"
}

function git_add_all_and_commit() {
    git add . && git commit -m "$1"
}

@test 'simple commit skip' {
    echo "$PWD"
    # should skip the b commit because we dont
    # specify path b.txt
    make_file a.txt && git_add_all_and_commit "a"
    make_file b.txt && git_add_all_and_commit "b"
    make_file c.txt && git_add_all_and_commit "c"

    [[ "$(num_commits)" == 3 ]]

    git checkout -b filterrepo
    git filter-repo --force --refs filterrepo --path a.txt --path c.txt

    # shouldnt exist anymore because we didnt include it
    [[ "$(num_commits)" == 2 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    [[ -f c.txt ]]
    gfr_ls_tree="$(git ls-tree HEAD)"
    gfr_log="$(git log --oneline --graph filterrepo)"

    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 3 ]]
    [[ -f b.txt ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --path a.txt --path c.txt > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_ls_tree="$(git ls-tree HEAD)"
    gitfilter_log="$(git log --oneline --graph gitfilter)"

    echo "$(git status)"
    echo "$(git log --oneline)"

    [[ "$(num_commits)" == 2 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    [[ -f c.txt ]]

    # if our tree matches, we have the exact same point in time
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]
    # we also check that our log is exactly the same
    [[ "$gfr_log" == "$gitfilter_log" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

@test 'simple merge skip' {
    echo "$PWD"
    # should skip the b commit, AND the resulting merge
    # because we dont specify path b.txt
    make_file a.txt && git_add_all_and_commit "a"
    git checkout -b m1
    make_file b.txt && git_add_all_and_commit "b"
    git checkout -
    make_file c.txt && git_add_all_and_commit "c"
    git merge --no-ff m1

    # 3 commits and a merge
    [[ "$(num_commits)" == 4 ]]

    git checkout -b filterrepo
    git filter-repo --force --refs filterrepo --path a.txt --path c.txt

    # shouldnt exist anymore because we didnt include it
    [[ "$(num_commits)" == 2 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    [[ -f c.txt ]]
    gfr_ls_tree="$(git ls-tree HEAD)"
    gfr_log="$(git log --oneline --graph filterrepo)"

    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 4 ]]
    [[ -f b.txt ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --path a.txt --path c.txt > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_ls_tree="$(git ls-tree HEAD)"
    gitfilter_log="$(git log --oneline --graph gitfilter)"

    echo "$(git status)"
    echo "$(git log --oneline)"

    [[ "$(num_commits)" == 2 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    [[ -f c.txt ]]

    # if our tree matches, we have the exact same point in time
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]
    # we also check that our log is exactly the same
    [[ "$gfr_log" == "$gitfilter_log" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}


# actually not simple!
# took me a day of debugging to figure out this is
# a necessary test case, TODO: make advanced test cases

# TODO: technically this isnt what i want to test....
# what I want to test is the effect of calling parents.rev()
# in filter_state.rs.
# its a difficult test case to reproduce but basically I want to test
# that the second merge commit gets filtered out becacuse
# it has fileops that already exist in the remaining parent
# however, its difficult to come up with a scenario where this
# merge commit has fileops different from the merge commit
# its trying to merge into...
# still, its a good test case because it tests some of the more
# advanced merge handling, but ideally itd test specifically
# about the ordering of the merges
@test 'merge commits get filtered if their contents already exists in their parents' {
    make_file a.txt
    make_file b.txt
    git_add_all_and_commit "a and b"
    git checkout -b original

    git checkout -b left1
    echo "a1" > a.txt && git add . && git commit -m "a1"
    git checkout -
    git checkout -b right1
    echo "a2" > a.txt && git add . && git commit -m "a2"
    echo "b1" > b.txt && git add . && git commit -m "b1"
    git checkout left1
    git merge -X ours --no-ff right1 --no-commit
    git commit -m "first merge that should be included"

    # important: contents should match the left commit
    a_contents="$(cat a.txt)"
    [[ $a_contents == "a1" ]]

    echo "ORIGINAL"
    git log --oneline --graph


    # This merge is a merge commit between 2 states:
    # a commit where we add C, and a commit where we had b1
    # if we merge this with the last merge commit we made,
    # we should:
    # 1. detect that C does not apply to our filter rules,
    # and thus this merge commit becomes a regular commit
    # with 1 parent.
    # 2. because its contents matches the parents contents, it should
    # be filtered out, ie: this became an empty merge commit
    git checkout right1
    make_file c.txt && git_add_all_and_commit "c"
    git checkout -
    git merge --no-ff right1 --no-commit
    git commit -m "weird merge that should be filtered"
    echo "AFTER WEIRD MERGE THAT SHOULD BE FILTERED"
    git log --oneline --graph

    git checkout -b gitfilter

    # before we filter, it will have the weird merge
    git_log="$(git log --oneline gitfilter)"
    [[ $git_log == *"weird merge"* ]]

    "$GITFILTERCLI" --branch gitfilter --path a.txt --path b.txt > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard

    echo "After gitfilter:"
    git log --oneline --graph

    # after filter, it should NOT have
    git_log="$(git log --oneline gitfilter)"
    [[ $git_log != *"weird merge"* ]]
}
