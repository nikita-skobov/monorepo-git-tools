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
        SEP="\\\\"
    else
        SEP="/"
    fi
}

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
    fi
    cd ..
    if [[ -d check ]]; then
        rm -rf check/
    fi
}

function setup() {
    test_folder="$BATS_TMPDIR/check"
    mkdir -p "$test_folder"
    BATS_TMPDIR="$test_folder"
    cd $test_folder
    set_seperator
    make_temp_repo test_remote_repo
    test_remote_repo="test_remote_repo"
    make_temp_repo test_remote_repo2
    test_remote_repo2="test_remote_repo2"
    cd $BATS_TMPDIR/test_remote_repo
}


# TODO: figure out how to properly prune
# FETCH_HEAD. I know you can do git fcsk FETCH_HEAD
# and get a list of all objects/commits, etc
# but i cant find a way to delete an object via its SHA
# @test 'size of .git should not change after run' {
    # repo_file_contents="
    # remote_repo=\"https://github.com/nikita-skobov/monorepo-git-tools\"
    # "
# 
    # size_git_before="$(du -s .git/*)"
    # echo "$repo_file_contents" > repo_file.sh
# 
    # [[ ! -f .git/FETCH_HEAD ]]
# 
    # run $PROGRAM_PATH check repo_file.sh
    # echo "$output"
    # size_git_after="$(du -s .git/*)"
    # echo "BEFORE:"
    # echo "$size_git_before"
    # echo "AFTER:"
    # echo "$size_git_after"
    # [[ $size_git_before == $size_git_after ]]
    # [[ ! -f .git/FETCH_HEAD ]]
# }

@test 'should report up to date if latest blob of both is the same' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    

    include=\"abc.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}

@test 'should work on a directory of repo files' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    mkdir -p repo_file_dir
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"


    include=\"abc.txt\"
    "
    echo "$repo_file_contents" > repo_file_dir/repo_file1.rf
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"


    include=\"xyz.txt\"
    "
    echo "$repo_file_contents" > repo_file_dir/repo_file2.rf

    # also to make sure it only grabs repo files:
    echo "not_a_rf.txt" > repo_file_dir/not_a_rf.txt

    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"

    run $PROGRAM_PATH check repo_file_dir
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
    [[ $output == *"repo_file1.rf"* ]]
    [[ $output == *"repo_file2.rf"* ]]
    [[ $output != *"not_a_rf.txt"* ]]
}

@test 'should work on a directory of repo files with any extension' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    mkdir -p repo_file_dir
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"


    include=\"abc.txt\"
    "
    echo "$repo_file_contents" > repo_file_dir/repo_file1.txt
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"


    include=\"xyz.txt\"
    "
    echo "$repo_file_contents" > repo_file_dir/repo_file2.txt

    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"

    run $PROGRAM_PATH check --all repo_file_dir
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
    [[ $output == *"repo_file1.txt"* ]]
    [[ $output == *"repo_file2.txt"* ]]
}

@test 'should work on a directory of repo files recursively' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    mkdir -p repo_file_dir
    mkdir -p repo_file_dir/recurse
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    
    
    include=\"abc.txt\"
    "
    echo "$repo_file_contents" > repo_file_dir/repo_file1.rf
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    
    
    include=\"xyz.txt\"
    "
    echo "$repo_file_contents" > repo_file_dir/repo_file2.rf
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    
    
    include=\"qqq.txt\"
    "
    echo "$repo_file_contents" > repo_file_dir/recurse/repo_file3.rf

    # also to make sure it only grabs repo files:
    echo "not_a_rf.txt" > repo_file_dir/not_a_rf.txt

    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"

    run $PROGRAM_PATH check --recursive repo_file_dir
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
    [[ $output == *"repo_file1.rf"* ]]
    [[ $output == *"repo_file2.rf"* ]]
    [[ $output == *"repo_file3.rf"* ]]
    [[ $output != *"not_a_rf.txt"* ]]
}

@test 'should report an update is necessary if theres one ahead' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # this will be the common point
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    # this will be the one ahead that should be reported
    echo "xyz" > abc.txt && git add abc.txt && git commit -m "xyz"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    commit_to_take="$(git log --oneline -n 1)"
    cd "$curr_dir"

    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    
    
    include = [\"abc.txt\", \"xyz.txt\"]
    "
    echo "$repo_file_contents" > repo_file.sh
    # simulate a point that is 'even' with the remote
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"$commit_to_take"* ]]
}

@test 'should report up-to-date if current blob isnt part of the include' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xy z" > "xy z.txt" && git add "xy z.txt" && git commit -m "xy z"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    commit_to_take="$(git log --oneline -n 1)"
    cd "$curr_dir"

    # we only care about abc.txt <-> abc.txt
    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    
    
    include=\"abc.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    # simulate a point that is 'even' with the remote
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}

@test 'should report up-to-date if current blob isnt part of the include_as' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abcd.txt && git add abcd.txt && git commit -m "abcd"
    echo "xy z" > "xy z.txt" && git add "xy z.txt" && git commit -m "xy z"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    commit_to_take="$(git log --oneline -n 1)"
    cd "$curr_dir"

    # we only care about abcd.txt <-> abc.txt
    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    [include_as]
    \"abc.txt\" = \"abcd.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    # simulate a point that is 'even' with the remote
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}

@test 'should report up-to-date if current blob is excluded' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abcd.txt && git add abcd.txt && git commit -m "abcd"
    echo "xy z" > "xy z.txt" && git add "xy z.txt" && git commit -m "xy z"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    commit_to_take="$(git log --oneline -n 1)"
    cd "$curr_dir"

    # we only care about abcd.txt <-> abc.txt
    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    
    
    exclude=[\"xy z.txt\", \"abcd.txt\"]
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}

@test 'should report up-to-date if current blob is excluded via path' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    mkdir -p "some path"
    mkdir -p "some path/lib"
    echo "xy z" > "some path/lib/xy z.txt" && git add "some path/" && git commit -m "some path"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    commit_to_take="$(git log --oneline -n 1)"
    cd "$curr_dir"

    # we only care about abcd.txt <-> abc.txt
    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    
    
    include = \"some path/\"
    exclude = \"some path/lib/\"
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}

@test 'should report take if current blob is part of include_as path' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p "some path"
    mkdir -p "some path/lib"
    # even though this has a different path, because its specified in the include_as
    # we should figure out that this blob applies to something in our local repo
    echo "abc" > "some path/lib/abc.txt" && git add "some path/lib/abc.txt" && git commit -m "abc"
    echo "xy z" > "some path/lib/xy z.txt" && git add "some path/" && git commit -m "some path"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    commit_to_take="$(git log --oneline -n 1)"
    cd "$curr_dir"

    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    [include_as]
    \" \" = \"some path/lib/\"
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"$commit_to_take"* ]]
}

@test 'should report up-to-date if current blob is part of include_as path but is excluded' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p "some path"
    mkdir -p "some path/lib"
    # even though this has a different path, because its specified in the include_as
    # we should figure out that this blob applies to something in our local repo
    echo "abc" > "some path/lib/abc.txt" && git add "some path/lib/abc.txt" && git commit -m "abc"
    echo "xy z" > "some path/lib/xy z.txt" && git add "some path/" && git commit -m "some path"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    commit_to_take="$(git log --oneline -n 1)"
    cd "$curr_dir"

    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    [include_as]
    \" \" = \"some path/lib/\"
    
    
    exclude=\"some path/lib/xy z.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}

@test '(local) should report take if current blob is part of include_as path' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p "some path"
    mkdir -p "some path/lib"
    # even though this has a different path, because its specified in the include_as
    # we should figure out that this blob applies to something in our local repo
    echo "abc" > "some path/lib/abc.txt" && git add "some path/lib/abc.txt" && git commit -m "abc"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    [include_as]
    \" \" = \"some path/lib/\"
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xy z" > "xy z.txt" && git add "xy z.txt" && git commit -m "xyz"
    commit_to_take="$(git log --oneline -n 1)"
    echo "LOCAL:"
    echo "$(git log --oneline)"
    run $PROGRAM_PATH check repo_file.sh --local
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"$commit_to_take"* ]]
}

@test '(local) should NOT report take if current blob is part of include_as path but is excluded' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p "some path"
    mkdir -p "some path/lib"
    # even though this has a different path, because its specified in the include_as
    # we should figure out that this blob applies to something in our local repo
    echo "abc" > "some path/lib/abc.txt" && git add "some path/lib/abc.txt" && git commit -m "abc"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    # the fact that remote has xyz.txt should be irrelevant
    # to this check
    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    [include_as]
    \" \" = \"some path/lib/\"


    exclude = \"xy z.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xy z" > "xy z.txt" && git add "xy z.txt" && git commit -m "xyz"
    commit_to_take="$(git log --oneline -n 1)"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check repo_file.sh --local
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}

@test 'should report take even if only one of the blobs applies to the repo file' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p "some path"
    mkdir -p "some path/lib"
    # even though this has a different path, because its specified in the include_as
    # we should figure out that this blob applies to something in our local repo
    echo "abc" > "some path/lib/abc.txt" && git add "some path/lib/abc.txt"
    # now this blob will not exist in the other repo,
    # so when we do a check, we should still detect this abc fork point
    # because this blob does not apply to the repo file:
    echo "qqq" > "some path/qqq.txt" && git add "some path/qqq.txt"
    git commit -m "multiple blobs"
    echo "REMOTE:"
    echo "$(git log --oneline)"
    cd "$curr_dir"

    repo_file_contents="
    [repo]
    remote = \"..$SEP$test_remote_repo2\"
    [include_as]
    \" \" = \"some path/lib/\"
    "
    echo "$repo_file_contents" > repo_file.sh
    # this should be detected as the fork point:
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "xy z" > "xy z.txt" && git add "xy z.txt" && git commit -m "xyz"
    commit_to_take="$(git log --oneline -n 1)"
    echo "LOCAL:"
    echo "$(git log --oneline)"
    run $PROGRAM_PATH check repo_file.sh --local
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"$commit_to_take"* ]]
}
