function setup() {
    # unfortunately this number needs to
    # be updated every time you add a test...
    LAST_TEST_NUMBER=1
    ORIGINAL_DIR="$PWD"
    test_folder="$BATS_TMPDIR/gfr-compat"


    if [[ ! -d "$test_folder" ]]; then
        mkdir -p "$test_folder"
        cd $test_folder
        git init
        git config --local user.name "test"
        git config --local user.email "test"
    fi

    BATS_TMPDIR="$test_folder"
}

function teardown() {
    # if the filtered.txt is left over after
    # the test, output it, so its visible in the error log
    cd $BATS_TMPDIR
    if [[ -f filtered.txt ]]; then
        echo "YOURS: =================="
        cat filtered.txt
        echo "THEIRS:=================="
        # ls -la .git/filter-repo
        cat .git/filter-repo/fast-export.filtered
    fi
    cd ..
    if [[ "$BATS_TEST_NUMBER" == "$LAST_TEST_NUMBER" ]]; then
        echo "Test done, removing temporary directory"
        # comment this out if you want to run the test just
        # once and then examine the files produced.
        if [[ -d gfr-compat ]]; then
            rm -rf gfr-compat/
        fi
    fi
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

# simple linear history with 2 files
# path include one of the files should ensure
# that the number of commits is only 1, and should
# only contain the 1 file that was explicitly included
@test 'simple path include rewrite' {
    make_file a.txt
    git_add_all_and_commit "a"
    make_file b.txt
    git_add_all_and_commit "b"

    [[ "$(num_commits)" == 2 ]]

    git checkout -b filterrepo
    # run it twice because the first time we just want
    # to generate the files to examine, and
    # the second time we actually run the filtering
    git filter-repo --force --refs filterrepo --path a.txt --dry-run
    git filter-repo --force --refs filterrepo --path a.txt
    # shouldnt exist anymore because we didnt include it
    [[ "$(num_commits)" == 1 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]

    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 2 ]]
    git checkout -b gitfilter
    "$GITFILTERCLI" --branch gitfilter --path a.txt > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force --quiet
    git log --oneline
    git status
    [[ "$(num_commits)" == 1 ]]
    ls
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    echo "DO WE GET HERE????"
    rm filtered.txt
}
