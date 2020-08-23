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
