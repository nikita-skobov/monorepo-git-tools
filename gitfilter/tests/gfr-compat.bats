function setup() {
    # unfortunately this number needs to
    # be updated every time you add a test...
    LAST_TEST_NUMBER=1
    ORIGINAL_DIR="$PWD"
    test_folder="$BATS_TMPDIR/gfr-compat"
    mkdir -p "$test_folder"
    BATS_TMPDIR="$test_folder"
    cd $test_folder
    # for the tests, we copy the react dir that the user
    # set to our own copy that we can manipulate/filter
    REACTPATH="$test_folder/react"
    TESTBRANCH="test-$BATS_TEST_NUMBER"
    CMPBRANCH="$TESTBRANCH-cmp"
    if [[ -z "$PATHTOREACTROOT" ]]; then
        echo "Cant run tests without a path to the react repo to test filter"
        exit 1
    fi
    if [[ ! -d "$REACTPATH" ]]; then
        echo "$GITFILTERCLI"
        echo "$REACTPATH"
        echo "copying react to $testfolder/react"
        cp -a "$PATHTOREACTROOT" "$test_folder/react"
    fi
    cd "$REACTPATH"
    git checkout master
}

function teardown() {
    if [[ "$BATS_TEST_NUMBER" == "$LAST_TEST_NUMBER" ]]; then
        echo "Test done, removing temporary directory"
        cd $BATS_TMPDIR
        cd ..
        # comment this out if you want to run the test just
        # once and then examine the files produced.
        if [[ -d gfr-compat ]]; then
            rm -rf gfr-compat/
        fi
    fi
}

function num_non_empty_commits() {
    num=0
    for i in $(git log --oneline --format='%h'); do
        # check if its an empty commit by
        # seeing if git show contains a diff, if its empty
        # text, then its an empty commit:
        if [[ $i == *"diff"* ]]; then
            num=$((num + 1))
        fi
    done
    echo "$num"
}

# get the git commit hash for the commit
# at the top of this branch, defaults to HEAD
# if $1 not provided
function hash_atop() {
    refname="${1:=HEAD}"
    git rev-parse --verify "$refname"
}

# get number of commits for the branch in $1
# or defaults to HEAD
function num_commits() {
    refname="${1:=HEAD}"
    git log --oneline "$refname" | wc -l
}

@test 'tree and non empty commits are the same for a real repo' {
    git checkout -b "$TESTBRANCH"
    git branch -v
    git filter-repo --force --refs "$TESTBRANCH" --path packages/react-dom/
    git branch "$CMPBRANCH" master

    gfr_ls_tree="$(git ls-tree HEAD)"
    gfr_num_non_empty="$(num_non_empty_commits)"

    "$GITFILTERCLI" --branch "$CMPBRANCH" --path packages/react-dom/ > filter1.txt
    cat filter1.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force --quiet
    git reset --hard

    gitfilter_ls_tree="$(git ls-tree HEAD)"
    gitfilter_num_non_empty="$(num_non_empty_commits)"

    git branch -v

    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]
    [[ "$gfr_num_non_empty" == "$gitfilter_num_non_empty" ]]
}

@test 'path rename works' {
    git checkout -b "$TESTBRANCH"
    git branch -v
    git filter-repo --path-rename packages/react-dom/src/: --force --refs "$TESTBRANCH"
    git branch "$CMPBRANCH" master

    gfr_ls_tree="$(git ls-tree HEAD)"
    gfr_num_non_empty="$(num_non_empty_commits)"

    "$GITFILTERCLI" --branch "$CMPBRANCH" --path-rename packages/react-dom/src/: > filter1.txt
    cat filter1.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force --quiet
    git reset --hard

    gitfilter_ls_tree="$(git ls-tree HEAD)"
    gitfilter_num_non_empty="$(num_non_empty_commits)"

    git branch -v

    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]
    [[ "$gfr_num_non_empty" == "$gitfilter_num_non_empty" ]]
}
