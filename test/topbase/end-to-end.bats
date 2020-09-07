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

@test 'if top is fast-forwardable onto base, topbase should result in the same as a ff merge' {
    echo "$(git branch -v)"
    echo "--"
    echo "$(git log --oneline)"
    git checkout -b top_branch
    echo "a" > a.txt && git add a.txt && git commit -m "a"

    # first we do a ff-merge to see what the history would look like
    git checkout master
    git checkout -b master-tmp
    git merge --ff-only top_branch
    ff_merge_log="$(git log --oneline)"

    # go back to top_branch
    git checkout top_branch
    git branch -D master-tmp

    # topbase current branch onto master
    # since we are directly ahead of master, and fast-forwardable
    # the rebase that topbase performs should just be the same as a ff merge
    run mgt topbase master
    echo "$output"
    [[ $status == 0 ]]
    echo "now gitlog:"
    echo "$(git log --oneline)"
    echo "should be:"
    echo "$ff_merge_log"
    [[ $ff_merge_log == "$(git log --oneline)" ]]
}
