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

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
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
    # run $PROGRAM_PATH check-updates repo_file.sh
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
    remote_repo=\"..$SEP$test_remote_repo2\"
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "LOCAL:"
    echo "$(git log --oneline)"


    run $PROGRAM_PATH check-updates repo_file.sh
    echo "$output"
    [[ $status == "0" ]]
    [[ $output == *"up to date"* ]]
}
