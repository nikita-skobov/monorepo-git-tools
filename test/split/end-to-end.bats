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
    source $BATS_TEST_DIRNAME/../../lib/helpers.bsc
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

@test 'can split in a local branch' {
    repo_file_contents="
    repo_name=\"doesnt_matter\"
    include_as=(
        \"this/path/will/be/created/\" \"\"
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

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh --merge-branch tmp1
    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/lib/file.txt ]]
}

@test 'can split in a remote_repo uri' {
    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \"\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh
    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]
}

@test 'can split in a remote_repo with a specific remote_branch' {
    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    remote_branch=\"test-branch\"
    include_as=(
        \"this/path/will/be/created/\" \"\"
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

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh
    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]
    [[ -f this/path/will/be/created/lib/test-branch-file.txt ]]
}


@test 'can specify an output branch with --output-branch' {
    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \"\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # branch should not exist at first:
    run branch_exists some-output-branch
    echo "$output"
    [[ $status -eq 1 ]]

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh --output-branch some-output-branch
    
    # now it should:
    run branch_exists some-output-branch
    [[ $status -eq 0 ]]
    # also we should be on that branch:
    run get_current_branch_name
    [[ $output == "some-output-branch" ]]
}

@test 'can specify an output branch with -o' {
    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \"\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # branch should not exist at first:
    run branch_exists some-output-branch
    echo "$output"
    [[ $status -eq 1 ]]

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh -o some-output-branch
    
    # now it should:
    run branch_exists some-output-branch
    [[ $status -eq 0 ]]
    # also we should be on that branch:
    run get_current_branch_name
    [[ $output == "some-output-branch" ]]
}


@test 'dont need a repo_name if providing a remote_repo uri (in)' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \"\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh

    echo "$output"

    [[ $status -eq 0 ]]
    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]

    # test that it makes the output branch name from
    # the remote_repo:
    run git rev-parse --abbrev-ref HEAD
    [[ $output == "test_remote_repo2-reverse" ]]
}

@test 'dont need a repo_name if providing a remote_repo uri (out)' {
    # from test_remote_repo, we split out the file test_remote_repo.txt
    # and into a repo called test_remote_repo2:
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"test_remote_repo.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh

    run $BATS_TEST_DIRNAME/git-split out repo_file.sh

    echo "$output"

    [[ $status -eq 0 ]]

    # test that it makes the output branch name from
    # the remote_repo:
    run git rev-parse --abbrev-ref HEAD
    [[ $output == "test_remote_repo2" ]]
}

@test 'can do a simple split in without using repo_file' {
    # we run git split in onto a remote_repo uri
    # instead of a repo file

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $BATS_TEST_DIRNAME/git-split in \
        file://$BATS_TMPDIR/test_remote_repo2 \
        --as this/path/will/be/created/

    echo "$output"
    [[ $status -eq 0 ]]

    # now it should exist:
    [[ -d this ]]

    # and just to be safe, check that the whole path to the files
    # is created:
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/test_remote_repo2.txt ]]

    # test that it makes the output branch name from
    # the remote_repo:
    run git rev-parse --abbrev-ref HEAD
    [[ $output == "test_remote_repo2-reverse" ]]
}

@test 'can split in and exclude files' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "rootfile1.txt" > rootfile1.txt
    echo "rootfile2.txt" > rootfile2.txt
    git add .
    git commit -m "adds 2 root files"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \"\"
    )
    exclude=\"rootfile1.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh

    echo "split in output:"
    echo "$output"

    # now it should exist:
    [[ -d this ]]

    # since we excluded rootfile1, it shouldnt be there
    # but rootfile2 should
    [[ -d this/path/will/be/created ]]
    [[ ! -f this/path/will/be/created/rootfile1.txt ]]
    [[ -f this/path/will/be/created/rootfile2.txt ]]
}

@test 'can split in and exclude folders' {
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
        \"this/path/will/be/created/\" \"\"
    )
    exclude=\"lib\"
    "

    echo "$repo_file_contents" > repo_file.sh

    # a directory called this should not exist at first
    [[ ! -d this ]]

    run $BATS_TEST_DIRNAME/git-split in repo_file.sh

    echo "split in output:"
    echo "$output"

    # now it should exist:
    [[ -d this ]]

    # since we excluded lib, it shouldnt be there
    # but rootfile1 should
    [[ -d this/path/will/be/created ]]
    [[ -f this/path/will/be/created/rootfile1.txt ]]
    [[ ! -d this/path/will/be/created/lib ]]
}
