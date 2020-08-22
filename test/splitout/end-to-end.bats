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

@test 'works for both folders and files' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=(
        \"a\"
        \"b/b1.txt\"
    )
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

    # since we only included a/ and b/b1.txt
    # b/b2.txt should not exist
    [[ -d a ]]
    [[ -d b ]]
    [[ -f a/a1.txt ]]
    [[ -f a/a2.txt ]]
    [[ -f b/b1.txt ]]
    [[ ! -f b/b2.txt ]]
}

@test 'works for recursive folders' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=(
        \"a/a1\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    mkdir -p a
    mkdir -p a/a1
    mkdir -p a/a1/a2
    mkdir -p a/c
    mkdir -p b
    echo "a" > a/a.txt
    echo "a1" > a/a1/a1.txt
    echo "a2" > a/a1/a2/a2.txt
    echo "ac" > a/c/c.txt
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

    # b should not exist
    # a/a1 should exist, but not a/c
    # and not a/a.txt
    [[ -d a ]]
    [[ -d a/a1 ]]
    [[ -d a/a1/a2 ]]
    [[ ! -d b ]]
    [[ ! -d a/c ]]
    [[ -f a/a1/a1.txt ]]
    [[ -f a/a1/a2/a2.txt ]]
}

@test 'can only include_as a single file' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"a.txt\"
        \"new_a.txt\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    echo "a" > a.txt
    echo "b" > b.txt
    git add a.txt
    git commit -m "a"
    git add b.txt
    git commit -m "b"

    run $PROGRAM_PATH split-out repo_file.sh --verbose

    echo "$output"
    [[ $status == "0" ]]

    # a.txt should be the only thing included
    # so b.txt should not exist, also a.txt
    # should been renamed to new_a.txt
    [[ -f new_a.txt ]]
    [[ ! -f a.txt ]]
    [[ ! -f b.txt ]]
}

@test 'can only include_as a single folder' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(
        \"a\"
        \"new_a\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    mkdir -p a
    mkdir -p a/a1
    mkdir -p b
    echo "a" > a/a.txt
    echo "a1" > a/a1/a1.txt
    echo "ac" > a/c.txt
    echo "b" > b/b.txt
    git add a
    git commit -m "a"
    git add b
    git commit -m "b"

    run $PROGRAM_PATH split-out repo_file.sh --verbose

    echo "$output"
    [[ $status == "0" ]]

    # b should not exist, and entirety of a/ should
    # be renamed to new_a/
    [[ -d new_a ]]
    [[ -d new_a/a1 ]]
    [[ ! -d a ]]
    [[ ! -d b ]]
    [[ -f new_a/c.txt ]]
    [[ -f new_a/a.txt ]]
    [[ -f new_a/a1/a1.txt ]]
}

@test 'can include_as to rename a nested folder but keep everything else' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include=\"a\"
    include_as=(
        \"a/old_a\"
        \"a/new_a\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    mkdir -p a
    mkdir -p a/old_a
    mkdir -p a/c
    echo "a" > a/a.txt
    echo "ac" > a/c/ac.txt
    echo "a1" > a/old_a/a1.txt
    echo "a2" > a/old_a/a2.txt
    git add a
    git commit -m "a"

    run $PROGRAM_PATH split-out repo_file.sh --verbose

    echo "$output"
    [[ $status == "0" ]]

    # b should not exist, and entirety of a/ should
    # be renamed to new_a/
    [[ -d a/new_a ]]
    [[ -d a/c ]]
    [[ ! -d a/old_a ]]
    [[ -f a/new_a/a1.txt ]]
    [[ -f a/new_a/a2.txt ]]
    [[ -f a/a.txt ]]
}

@test 'can include_as include and exclude a specific directory structure' {
    repo_file_contents="
    remote_repo=\"$BATS_TMPDIR/test_remote_repo2\"
    include_as=(\"a/a1\" \"lib\")
    exclude=(
        \"a/a1/b\"
        \"a/a1/a1.txt\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    mkdir -p a
    mkdir -p a/a1
    mkdir -p a/a1/b
    mkdir -p a/a1/c
    echo "a1" > a/a1/a1.txt
    echo "ac" > a/a1/c/ac.txt
    echo "a2" > a/a1/a2.txt
    echo "b" > a/a1/b/b.txt
    git add a
    git commit -m "a"

    run $PROGRAM_PATH split-out repo_file.sh --verbose

    echo "$output"
    echo "$(find -L .)"
    [[ $status == "0" ]]

    # b should not exist, and entirety of a/ should
    # be renamed to new_a/
    [[ -d lib ]]
    [[ -d lib/c ]]
    [[ ! -d a ]]
    [[ ! -d lib/b ]]
    [[ ! -f lib/a1.txt ]]
    [[ -f lib/c/ac.txt ]]
    [[ -f lib/a2.txt ]]
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

