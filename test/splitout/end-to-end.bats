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

function set_seperator() {
    # I wanna use these tests for both windows (git bash)
    # and linux, so I need to change the separator
    if [[ -d /c/ ]]; then
        SEP="\\"
    else
        SEP="/"
    fi
}

function setup() {
    set_seperator
    make_temp_repo test_remote_repo
    test_remote_repo="test_remote_repo"
    make_temp_repo test_remote_repo2
    test_remote_repo2="test_remote_repo2"
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

@test 'capable of only including certain files' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"a.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "repo file contents:"
    cat repo_file.sh

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

@test 'repo_file doesnt need a remote_repo if --output-branch provided' {
    repo_file_contents="
    include=\"b.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "repo file contents:"
    cat repo_file.sh

    echo "b" > b.txt && git add b.txt && git commit -m "b"

    run $PROGRAM_PATH split-out repo_file.sh --verbose --output-branch my-branch
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "my-branch" ]]
}

@test '--output-branch should override repo_name' {
    repo_file_contents="
    include=\"b.txt\"
    repo_name=\"somename\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "repo file contents:"
    cat repo_file.sh

    echo "b" > b.txt && git add b.txt && git commit -m "b"

    run $PROGRAM_PATH split-out repo_file.sh --verbose --output-branch my-branch
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "my-branch" ]]
}

@test '--output-branch should override remote_repo' {
    repo_file_contents="
    include=\"b.txt\"
    remote_repo=\"..$SEP$test_remote_repo2\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "repo file contents:"
    cat repo_file.sh

    echo "b" > b.txt && git add b.txt && git commit -m "b"

    run $PROGRAM_PATH split-out repo_file.sh --verbose --output-branch my-branch
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "my-branch" ]]
}

@test 'capable of only including certain folders' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
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
    remote_repo=\"..$SEP$test_remote_repo2\"
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
    remote_repo=\"..$SEP$test_remote_repo2\"
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

@test 'can split out to a specific output branch' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
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

    run $PROGRAM_PATH split-out repo_file.sh --verbose --output-branch newbranch1

    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == *"newbranch1"* ]]

    # a.txt should be the only thing included
    # so b.txt should not exist, also a.txt
    # should been renamed to new_a.txt
    [[ -f new_a.txt ]]
    [[ ! -f a.txt ]]
    [[ ! -f b.txt ]]
}

@test 'can only include_as a single file' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
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
    remote_repo=\"..$SEP$test_remote_repo2\"
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

@test 'can only include_as a single to root' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"a/\"
        \" \"
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
    [[ ! -d a ]]
    [[ ! -d b ]]
    [[ -f c.txt ]]
    [[ -f a.txt ]]
    [[ -f a1/a1.txt ]]
}

@test 'can include_as to rename a nested folder but keep everything else' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
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
    remote_repo=\"..$SEP$test_remote_repo2\"
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
    remote_repo=\"..$SEP$test_remote_repo2\"
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

@test 'can optionally rebase new branch onto original' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=(\"lib/\" \"test_remote_repo.txt\")
    "
    echo "$repo_file_contents" > repo_file.sh

    mkdir -p lib/
    echo "libfile1.txt" > lib/libfile1.txt
    git add lib/libfile1.txt && git commit -m "libfile1"

    run $PROGRAM_PATH split-out repo_file.sh -r --verbose
    echo "$output"
    echo "$(git branch -v)"
    echo -e "\n$(git branch --show-current):"
    echo "$(git log --oneline)"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    output_log="$(git log --oneline)"
    output_commits="$(git log --oneline | wc -l)"
    echo ""

    # we test that the number of commits is now the number that we made in our local
    # repo (2: the original, and the libfile) plus the initial commit of test_remote_repo2
    # so should be three
    [[ "$output_commits" == "3" ]]
    [[ "$output_log" == *"libfile1"* ]]
    [[ "$output_log" == *"initial commit for test_remote_repo"* ]]
    [[ "$output_log" == *"initial commit for test_remote_repo2"* ]]
}

@test 'rebasing new branch onto original should not leave temporary branch' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=(\"lib/\" \"test_remote_repo.txt\")
    "
    echo "$repo_file_contents" > repo_file.sh

    mkdir -p lib/
    echo "libfile1.txt" > lib/libfile1.txt
    git add lib/libfile1.txt && git commit -m "libfile1"

    run $PROGRAM_PATH split-out repo_file.sh -r --verbose
    echo "$output"
    echo "$(git branch -v)"
    echo -e "\n$(git branch --show-current):"
    echo "$(git log --oneline)"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    output_log="$(git log --oneline)"
    output_commits="$(git log --oneline | wc -l)"
    echo ""
    echo "$(git branch)"
    expected_branches=$(echo -e "  master\n* test_remote_repo2")
    echo "expected branches:"
    echo "$expected_branches"
    echo "branches:"
    echo "$(git branch)"
    [[ "$(git branch)" == $expected_branches ]]
}

@test 'can topbase new branch onto original branch' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=(\"lib/\")
    "
    echo "$repo_file_contents" > repo_file.sh

    mkdir -p lib/
    echo "libfile1.txt" > lib/libfile1.txt
    git add lib/libfile1.txt && git commit -m "libfile1"

    # make the same 'contribution' on the remote repo
    # this mimics a scenario where you have previously
    # contributed to that repo, so the next time you contribute,
    # topbase will only add your most recent commits
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib/
    echo "libfile1.txt" > lib/libfile1.txt
    git add lib/libfile1.txt && git commit -m "libfile1"
    cd "$curr_dir"

    # this is the recent contribution that will be topbased
    echo "libfile1-mod" >> lib/libfile1.txt
    git add lib/libfile1.txt && git commit -m "libfile1mod"

    run $PROGRAM_PATH split-out repo_file.sh -t --verbose
    echo "$output"
    echo "$(git branch -v)"
    echo -e "\n$(git branch --show-current):"
    echo "$(git log --oneline)"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    output_log="$(git log --oneline)"
    output_commits="$(git log --oneline | wc -l)"
    echo ""
    echo "$(git branch)"
    # output commits should be 3:
    # 1 initial commit of remote,
    # the initial contribution "libfile1"
    # and then the topbased commit "libfile1mod"
    [[ "$output_commits" == 3 ]]
}

@test '--topbase should not say success if there were rebase merge conflicts' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=(\"lib/\")
    "
    echo "$repo_file_contents" > repo_file.sh

    mkdir -p lib/
    echo "libfile1.txt" > lib/libfile1.txt
    git add lib/libfile1.txt && git commit -m "libfile1"
    echo "conflict" > lib/libfile1.txt && git add lib/libfile1.txt && git commit -m "conflict"

    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib/
    echo "libfile1.txt" > lib/libfile1.txt
    git add lib/libfile1.txt && git commit -m "libfile1"
    echo "test" > lib/libfile1.txt && git add lib/libfile1.txt && git commit -m "libfile1mod"
    cd "$curr_dir"

    run $PROGRAM_PATH split-out repo_file.sh --topbase --verbose
    echo "$output"
    echo "$(git status)"
    [[ $output != "Success" ]]
    [[ $status == "1" ]]
    [[ "$(git status)" == *"rebase in progress"* ]]
}
