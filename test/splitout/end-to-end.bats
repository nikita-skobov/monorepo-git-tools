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
    echo "SETUP!"
    make_temp_repo test_remote_repo
    make_temp_repo test_remote_repo2
    cd $BATS_TMPDIR/test_remote_repo
}

function teardown() {
    echo "TEARDOWN!"
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
    fi
}

@test 'capable of only including certain files' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"a.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh

    echo "b" > b.txt
    echo "a" > a.txt
    git add a.txt
    git commit -m "a"
    git add b.txt
    git commit -m "b"

    [[ -f a.txt ]]
    [[ -f b.txt ]]

    run $PROGRAM_PATH split-out repo_file.sh --verbose

    echo "$output"
    [[ $status == "0" ]]

    # since we only included a.txt
    # b.txt should not exist
    [[ -f a.txt ]]
    [[ ! -f b.txt ]]
}

@test 'capable of only including certain folders' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"a\"
    "

    echo "$repo_file_contents" > repo_file.sh

    mkdir -p a
    mkdir -p b
    echo "a1" > a/a1.txt
    echo "a2" > a/a2.txt
    echo "b1" > b/b1.txt
    echo "b2" > b/b2.txt
    git add a
    git commit -m "a"
    git add b
    git commit -m "b"

    [[ -d a ]]
    [[ -d b ]]

    run $PROGRAM_PATH split-out repo_file.sh --verbose

    echo "$output"
    [[ $status == "0" ]]

    # since we only included a
    # b dir should not exist
    [[ -d a ]]
    [[ ! -d b ]]
    [[ -f a/a1.txt ]]
    [[ -f a/a2.txt ]]
}

@test 'dont need a repo_name if providing a remote_repo uri (out)' {
    # from test_remote_repo, we split out the file test_remote_repo.txt
    # and into a repo called test_remote_repo2:
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"test_remote_repo.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH split-out repo_file.sh --verbose

    echo "$output"

    [[ $status -eq 0 ]]

    # test that it makes the output branch name from
    # the remote_repo:
    run git rev-parse --abbrev-ref HEAD
    [[ $output == "test_remote_repo2" ]]
}

