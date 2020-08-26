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

@test 'can split-in-as a remote uri without a repo_file' {
    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $PROGRAM_PATH split-in-as "$BATS_TMPDIR/test_remote_repo2" --as this/path/will/be/created/ --verbose
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

    run $PROGRAM_PATH split-in-as "$BATS_TMPDIR/test_remote_repo2" --as abc/ --verbose --rebase
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
