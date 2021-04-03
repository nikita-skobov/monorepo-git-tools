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

# simple linear history with 2 files
# path include one of the files should ensure
# that the number of commits is only 1, and should
# only contain the 1 file that was explicitly included
@test 'simple path include rewrite' {
    echo "$PWD"
    make_file a.txt
    git_add_all_and_commit "a"
    make_file b.txt
    git_add_all_and_commit "b"

    [[ "$(num_commits)" == 2 ]]

    git checkout -b filterrepo
    git filter-repo --force --refs filterrepo --path a.txt --dry-run
    cat .git/filter-repo/fast-export.filtered | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard

    # shouldnt exist anymore because we didnt include it
    [[ "$(num_commits)" == 1 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    gfr_hash="$(hash_atop)"


    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 2 ]]
    [[ -f b.txt ]]
    git checkout -b gitfilter
    [[ -f b.txt ]]

    "$GITFILTERCLI" --branch gitfilter --path a.txt > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_hash="$(hash_atop)"

    [[ "$(num_commits)" == 1 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]

    # test that our rewrite hash matches their rewrite hash
    [[ "$gfr_hash" == "$gitfilter_hash" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

# simple linear history with 2 folders and 3 files
# basically the same as the above test, but check if path include
# works for folders
@test 'simple path folder include rewrite' {
    mkdir -p folder_a
    mkdir -p folder_b
    make_file folder_a/a.txt
    git_add_all_and_commit "a"
    make_file folder_b/b.txt
    git_add_all_and_commit "b"
    make_file folder_a/unused.txt
    git_add_all_and_commit "unused"

    [[ "$(num_commits)" == 3 ]]

    git checkout -b filterrepo
    git filter-repo --force --refs filterrepo --path folder_b/ --dry-run
    cat .git/filter-repo/fast-export.filtered | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard

    # shouldnt exist anymore because we didnt include it
    [[ "$(num_commits)" == 1 ]]
    [[ ! -d folder_a/ ]]
    [[ -d folder_b/ ]]
    [[ ! -f folder_a/a.txt ]]
    [[ -f folder_b/b.txt ]]
    gfr_hash="$(hash_atop)"


    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 3 ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --path folder_b/ > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_hash="$(hash_atop)"

    [[ "$(num_commits)" == 1 ]]
    [[ ! -d folder_a/ ]]
    [[ -d folder_b/ ]]
    [[ ! -f folder_a/a.txt ]]
    [[ -f folder_b/b.txt ]]
    # test that our rewrite hash matches their rewrite hash
    [[ "$gfr_hash" == "$gitfilter_hash" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

@test 'merge included when it contains part of the include path' {
    mkdir -p folder_a
    make_file folder_a/a.txt
    git_add_all_and_commit "a"
    checkout_from="$(hash_atop)"
    make_file unwanted.txt
    git_add_all_and_commit "unwanted"
    git branch tmp1 "$checkout_from"
    git checkout tmp1
    echo "a2" >> folder_a/a.txt
    git_add_all_and_commit "a2"
    git checkout master
    git merge --no-ff --commit tmp1

    git log --oneline

    # 3 commits and 1 merge commit
    [[ "$(num_commits)" == 4 ]]

    git checkout -b filterrepo
    git filter-repo --force --refs filterrepo --path folder_a/ --dry-run
    cat .git/filter-repo/fast-export.filtered | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard

    git log --oneline

    # it should include the merge commit
    # because part of what was merged included folder_a/
    [[ "$(num_commits)" == 3 ]]
    [[ -d folder_a/ ]]
    [[ -f folder_a/a.txt ]]
    [[ ! -f unwanted.txt ]]
    gfr_hash="$(hash_atop)"

    # now we test our version
    git checkout master
    [[ "$(num_commits)" == 4 ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --path folder_a/ > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    [[ "$(num_commits)" == 3 ]]
    [[ -d folder_a/ ]]
    [[ -f folder_a/a.txt ]]
    [[ ! -f unwanted.txt ]]
    gfr_hash="$(hash_atop)"

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

@test 'merge NOT included when it DOESNT contain part of the include path' {
    mkdir -p folder_a
    make_file folder_a/a.txt
    git_add_all_and_commit "a"
    make_file unwanted.txt
    git_add_all_and_commit "unwanted"
    git checkout -b tmp1
    echo "unwanted2" >> unwanted.txt
    git_add_all_and_commit "unwanted2"
    git checkout master
    echo "a2" >> folder_a/a.txt
    git_add_all_and_commit "a2"
    git merge --no-ff --commit tmp1

    git log --oneline

    # 4 commits and 1 merge commit
    [[ "$(num_commits)" == 5 ]]

    git checkout -b filterrepo
    # wed like to do a dry run here so we can examine
    # the filtered log, but git filter-repo does more than just use the
    # filtered output piped into git fast-import. This is one of
    # those cases where it does some extra stuff which is hard to emulate
    # in a script. so for this test, we just run the command end to end:
    git filter-repo --force --refs filterrepo --path folder_a/

    git log --oneline
    git show HEAD

    # should not include the merge commit
    [[ "$(num_commits)" == 2 ]]
    [[ -d folder_a/ ]]
    [[ -f folder_a/a.txt ]]
    [[ ! -f unwanted.txt ]]
    gfr_hash="$(hash_atop)"

    # now we test our version
    git checkout master
    [[ "$(num_commits)" == 5 ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --path folder_a/ > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    [[ "$(num_commits)" == 2 ]]
    [[ -d folder_a/ ]]
    [[ -f folder_a/a.txt ]]
    [[ ! -f unwanted.txt ]]
    gitfilter_hash="$(hash_atop)"

    # test that our rewrite hash matches their rewrite hash
    [[ "$gfr_hash" == "$gitfilter_hash" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

@test 'simple exlcude works' {
    mkdir -p folder_a
    mkdir -p folder_b
    make_file folder_a/a.txt
    git_add_all_and_commit "a"
    make_file folder_b/b.txt
    git_add_all_and_commit "b"
    echo "a2" >> folder_a/a.txt
    git_add_all_and_commit "a2"
    echo "a3" >> folder_a/a.txt
    git_add_all_and_commit "a3"
    echo "b2" >> folder_b/b.txt
    git_add_all_and_commit "b2"

    [[ "$(num_commits)" == 5 ]]

    git checkout -b filterrepo
    git filter-repo --force --refs filterrepo --invert-paths --path folder_a/ --dry-run
    cat .git/filter-repo/fast-export.filtered | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard

    # shouldnt exist anymore because we excluded it
    [[ "$(num_commits)" == 2 ]]
    [[ ! -d folder_a/ ]]
    [[ -d folder_b/ ]]
    [[ ! -f folder_a/a.txt ]]
    [[ -f folder_b/b.txt ]]
    gfr_hash="$(hash_atop)"

    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 5 ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --default-include --exclude-path folder_a/ > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_hash="$(hash_atop)"

    git log --oneline

    [[ "$(num_commits)" == 2 ]]
    [[ ! -d folder_a/ ]]
    [[ -d folder_b/ ]]
    [[ ! -f folder_a/a.txt ]]
    [[ -f folder_b/b.txt ]]
    # test that our rewrite hash matches their rewrite hash
    [[ "$gfr_hash" == "$gitfilter_hash" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}
