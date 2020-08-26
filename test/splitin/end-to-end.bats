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

function setup() {
    make_temp_repo test_remote_repo
    make_temp_repo test_remote_repo2
    cd $BATS_TMPDIR/test_remote_repo
}

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
    fi
}

@test 'can split in a remote_repo uri' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \" \"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    echo "$output"
    [[ $status == "0" ]]

    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]
}

@test 'can split in a local branch' {
    repo_file_contents="
    include_as=(
        \"this/path/will/be/created/\" \" \"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    git checkout -b tmp1
    mkdir -p lib
    echo "libfiletext" > lib/file.txt
    git add lib/
    git commit -m "lib commit"
    git checkout master

    run $PROGRAM_PATH split-in --verbose --input-branch tmp1 repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/lib/file.txt ]]
}

@test 'can split in to a specific output branch' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \" \"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $PROGRAM_PATH split-in repo_file.sh --verbose -o newbranch1
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == *"newbranch1"* ]]

    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]
}

@test 'can split in a remote_repo with a specific remote_branch' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    remote_branch=\"test-branch\"
    include_as=(
        \"this/path/will/be/created/\" \" \"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    cd $BATS_TMPDIR/test_remote_repo2
    git checkout -b test-branch
    mkdir -p lib
    echo "libfiletext" > lib/test-branch-file.txt
    git add lib/
    git commit -m "lib commit"
    git checkout master
    cd -

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    echo "$output"
    [[ $status == "0" ]]
    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]
    [[ -f this/path/will/be/created/lib/test-branch-file.txt ]]
}

@test 'can include only parts of remote repos' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"locallib/\" \"lib/\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"

    # since we excluded lib, it shouldnt be there
    # but rootfile1 should
    [[ -d locallib ]]

    [[ -f locallib/libfile1.txt ]]
    [[ -f locallib/libfile2.txt ]]
    [[ ! -f libfile1.txt ]]
    [[ ! -f rootfile1.txt ]]
}

@test 'can include without renaming' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"lib/libfile1.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"


    [[ -f lib/libfile1.txt ]]
    [[ ! -f lib/libfile2.txt ]]
    [[ ! -f rootfile1.txt ]]
    [[ ! -f test_remote_repo2.txt ]]
}

@test 'can include folders without renaming' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    mkdir -p src
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    echo "srcfile1.txt" > src/srcfile1.txt
    echo "srcfile2.txt" > src/srcfile2.txt
    git add lib
    git commit -m "adds 2 lib files and 1 root file"
    git add src
    git commit -m "adds src"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=(\"src/\" \"lib\")
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"


    [[ -f lib/libfile1.txt ]]
    [[ -f lib/libfile2.txt ]]
    [[ -f src/srcfile1.txt ]]
    [[ -f src/srcfile2.txt ]]
    [[ ! -f rootfile1.txt ]]
    [[ ! -f test_remote_repo2.txt ]]
}

@test 'can include a folder and exclude a subfolder' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    mkdir -p lib/test
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    echo "testfile1.txt" > lib/test/testfile1.txt
    echo "testfile2.txt" > lib/test/testfile2.txt
    git add .
    git commit -m "adds tests"

    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"lib/\"
    exclude=\"lib/test/\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"


    [[ -f lib/libfile1.txt ]]
    [[ -f lib/libfile2.txt ]]
    [[ ! -d lib/test ]]
    [[ ! -f lib/test/testfile1.txt ]]
    [[ ! -f lib/test/testfile2.txt ]]
    [[ ! -f test_remote_repo2.txt ]]
    [[ ! -f rootfile1.txt ]]
}

# rebase onto means the changes stay on the new branch, but
# it uses the original branch as the upstream branch to compare with
@test 'can optionally rebase the new branch onto original branch' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    master_commits="$(git log --oneline | wc -l)"
    made_commits=0
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    ((made_commits += 1))
    cd "$curr_dir"

    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"lib/\"
    "

    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH split-in repo_file.sh -r --verbose
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

    [[ -f lib/libfile1.txt ]]
    [[ -f lib/libfile2.txt ]]
    [[ ! -f rootfile1.txt ]]
    # since we specified to rebase, this file should exist
    # because it existed in master, and we rebased our new branch on top of master
    [[ -f test_remote_repo.txt ]]
}
