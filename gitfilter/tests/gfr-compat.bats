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
        # if [[ -d gfr-compat ]]; then
        #     rm -rf gfr-compat/
        # fi
    fi
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


# this test compares the output of the fast-export file
# before it is fed into fast-import
# however, I think its more desirable to actually run the
# import, and compare the repository commits/history.
# this is because the git filter-repo adds many unnecessary reset lines
# and I want to see if it would produce the same history without those.
# @test 'simple path include' {
#     echo "$ORIGINAL_DIR"
#     echo "$GITFILTERCLI"
#     echo "$PATHTOREACTROOT"

#     cd "$PATHTOREACTROOT"
#     # this will generate the fast export comparison file:
#     # .git/filter-repo/fast-export.filtered
#     git filter-repo --force --dry-run --refs master --path packages/react-dom/
#     COMPARE_PATH="$PATHTOREACTROOT/.git/filter-repo/fast-export.filtered"
#     [[ -f $COMPARE_PATH ]]
#     COMPARE_PATH_ACTUAL="$BATS_TMPDIR/gitfiltercli.output"
#     "$GITFILTERCLI" --path packages/react-dom/ > "$COMPARE_PATH_ACTUAL"

#     echo "Comparing $COMPARE_PATH to $COMPARE_PATH_ACTUAL"
#     echo "File sizes:"
#     echo "$(wc -c $COMPARE_PATH)"
#     echo "$(wc -c $COMPARE_PATH_ACTUAL)"
#     # for debugging:
#     # cp "$COMPARE_PATH" "$COMPARE_PATH_ACTUAL" "$ORIGINAL_DIR"
#     cmp -s "$COMPARE_PATH" "$COMPARE_PATH_ACTUAL"
# }

@test 'simple path include rewrite' {
    git checkout -b "$TESTBRANCH"
    git branch -v
    git filter-repo --force --refs "$TESTBRANCH" --path packages/react-dom/
    git branch "$CMPBRANCH" master
    echo "outputting to: $ORIGINAL_DIR/plsexamine.txt"
    "$GITFILTERCLI" --branch "$CMPBRANCH" --path packages/react-dom/ > "$ORIGINAL_DIR/plsexamine.txt"
    cat "$ORIGINAL_DIR/plsexamine.txt" | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force --quiet
    # git filter-repo --force --refs "$CMPBRANCH" --path packages/react-dom/
    git branch -v

    master_commits="$(num_commits master)"
    cmp_commits="$(num_commits $CMPBRANCH)"
    test_commits="$(num_commits $TESTBRANCH)"
    echo "master: $master_commits"
    echo "cmp: $cmp_commits"
    echo "test: $test_commits"

    [[ "$(hash_atop $TESTBRANCH)" == "$(hash_atop $CMPBRANCH)" ]]
    [[ "$(hash_atop $CMPBRANCH)" != "$(hash_atop master)" ]]
    [[ $cmp_commits -lt $master_commits ]]
}
