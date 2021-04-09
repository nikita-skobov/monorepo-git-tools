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
    gfr_ls_tree="$(git ls-tree HEAD)"

    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 2 ]]
    [[ -f b.txt ]]
    git checkout -b gitfilter
    [[ -f b.txt ]]

    "$GITFILTERCLI" --branch gitfilter --path a.txt > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_ls_tree="$(git ls-tree HEAD)"

    [[ "$(num_commits)" == 1 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]

    # if our tree matches, we have the exact same point in time
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]

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
    gfr_ls_tree="$(git ls-tree HEAD)"


    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 3 ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --path folder_b/ > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_ls_tree="$(git ls-tree HEAD)"

    [[ "$(num_commits)" == 1 ]]
    [[ ! -d folder_a/ ]]
    [[ -d folder_b/ ]]
    [[ ! -f folder_a/a.txt ]]
    [[ -f folder_b/b.txt ]]
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

@test 'empty merge commits should be removed' {
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
    git filter-repo --force --refs filterrepo --path folder_a/
    git log --oneline --decorate --graph

    # there should only be a, and a2, because
    # the merge commit is unecessary and should be pruned
    [[ "$(num_commits)" == 2 ]]
    [[ -d folder_a/ ]]
    [[ -f folder_a/a.txt ]]
    [[ ! -f unwanted.txt ]]
    gfr_ls_tree="$(git ls-tree HEAD)"
    echo "gitfilterrepo ls-tree:"
    echo "$gfr_ls_tree"

    # now we test our version
    git checkout master
    [[ "$(num_commits)" == 4 ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --path folder_a/ > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    git log --oneline --decorate --graph
    gitfilter_ls_tree="$(git ls-tree HEAD)"
    echo "gitfilter ls-tree:"
    echo "$gitfilter_ls_tree"

    [[ "$(num_commits)" == 2 ]]
    [[ -d folder_a/ ]]
    [[ -f folder_a/a.txt ]]
    [[ ! -f unwanted.txt ]]
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]


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
    git filter-repo --force --refs filterrepo --invert-paths --path folder_a/

    # shouldnt exist anymore because we excluded it
    [[ "$(num_commits)" == 2 ]]
    [[ ! -d folder_a/ ]]
    [[ -d folder_b/ ]]
    [[ ! -f folder_a/a.txt ]]
    [[ -f folder_b/b.txt ]]
    gfr_ls_tree="$(git ls-tree HEAD)"

    # now try doing the same thing but using our new tool:
    git checkout master
    [[ "$(num_commits)" == 5 ]]
    git checkout -b gitfilter

    "$GITFILTERCLI" --branch gitfilter --default-include --exclude-path folder_a/ > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_ls_tree="$(git ls-tree HEAD)"
    git log --oneline

    [[ "$(num_commits)" == 2 ]]
    [[ ! -d folder_a/ ]]
    [[ -d folder_b/ ]]
    [[ ! -f folder_a/a.txt ]]
    [[ -f folder_b/b.txt ]]
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

# here we have a history like:
# C  a.txt
# B  b.txt -> a.txt
# A  b.txt
# we make sure that our history starts at B, and doesnt consider
# that a.txt is renamed, but rather as the initial commit
@test 'rename file doesnt get detected as the initial commit' {
    make_file b.txt
    git_add_all_and_commit "b"
    git mv b.txt a.txt
    git commit -m "rename to a"
    # just in case, lets look at what that looks like:
    echo "-----"
    git show HEAD
    echo "-----"
    echo "aa" >> a.txt
    git_add_all_and_commit "a2"

    git checkout -b filterrepo
    git filter-repo --force --refs filterrepo --path a.txt --dry-run
    echo "ORIGINAL"
    cat .git/filter-repo/fast-export.original
    echo "FILTERED"
    cat .git/filter-repo/fast-export.filtered
    git filter-repo --force --refs filterrepo --path a.txt
    echo "gitfilterrepo:"
    git log --oneline

    [[ "$(num_commits)" == 2 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    gfr_ls_tree="$(git ls-tree HEAD)"

    git checkout master
    [[ "$(num_commits)" == 3 ]]
    git checkout -b gitfilter
    "$GITFILTERCLI" --branch gitfilter --path a.txt > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    echo "gitfilter:"
    git log --oneline
    
    gitfilter_ls_tree="$(git ls-tree HEAD)"
    [[ "$(num_commits)" == 2 ]]
    [[ ! -f b.txt ]]
    [[ -f a.txt ]]
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]

    cat filtered.txt

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

@test 'can rename folders to root' {
    mkdir -p folder_a/
    make_file folder_a/a.txt
    git_add_all_and_commit "a"

    git checkout -b filterrepo
    git filter-repo --path-rename folder_a/: --refs filterrepo --force
    [[ ! -d folder_a/ ]]
    [[ -f a.txt ]]
    [[ "$(num_commits)" == 1 ]]
    gfr_ls_tree="$(git ls-tree HEAD)"


    git checkout master
    git checkout -b gitfilter
    "$GITFILTERCLI" --branch gitfilter --path-rename folder_a/: > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_ls_tree="$(git ls-tree HEAD)"

    [[ ! -d folder_a/ ]]
    [[ -f a.txt ]]
    [[ "$(num_commits)" == 1 ]]
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]

    cat filtered.txt

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}

@test 'can handle spaces' {
    mkdir -p "folder one/"
    make_file "folder one/a.txt"
    git_add_all_and_commit "a"

    git checkout -b filterrepo
    git filter-repo --path-rename "folder one/:nospace/" --refs filterrepo --force
    [[ ! -d "folder one/" ]]
    [[ -f nospace/a.txt ]]
    [[ "$(num_commits)" == 1 ]]
    gfr_ls_tree="$(git ls-tree HEAD)"


    git checkout master
    git checkout -b gitfilter
    "$GITFILTERCLI" --branch gitfilter --path-rename "folder one/:nospace/" > filtered.txt
    cat filtered.txt | git -c core.ignorecase=false fast-import --date-format=raw-permissive --force
    git reset --hard
    gitfilter_ls_tree="$(git ls-tree HEAD)"

    [[ ! -d "folder one/" ]]
    [[ -f nospace/a.txt ]]
    [[ "$(num_commits)" == 1 ]]
    [[ "$gfr_ls_tree" == "$gitfilter_ls_tree" ]]

    cat filtered.txt

    rm filtered.txt
    git checkout master
    git branch -D gitfilter filterrepo
}
