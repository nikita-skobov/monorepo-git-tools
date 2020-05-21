function setup() {
    cd $BATS_TMPDIR
    mkdir -p test_remote_repo
    cd test_remote_repo
    if [[ ! -d .git ]]; then
        git init
        git config --local user.email "temp"
        git config --local user.name "temp"
        echo "sometext" > somefile.txt
        git add somefile.txt
        git commit -m "initial commit"
    fi
    echo "$PWD"
}

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
}


@test 'can split in a remote github repo via username and repo_name into a subfolder of current repo' {
    # https://github.com/nikita-skobov/github-actions-tutorial
    repo_file_contents="
    repo_name=\"github-actions-tutorial\"
    username=\"nikita-skobov\"
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
    [[ -f this/path/will/be/created/LICENSE ]]
    [[ -f this/path/will/be/created/README.md ]]
}

@test 'can split in a local branch into a subfolder of current repo' {
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
