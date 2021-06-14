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

function setup() {
    test_folder="$BATS_TMPDIR/topbase"
    mkdir -p "$test_folder"
    BATS_TMPDIR="$test_folder"
    cd $test_folder
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
    cd ..
    if [[ -d topbase ]]; then
        rm -rf topbase/
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

@test 'should use current branch as the top branch by default' {
    git checkout -b top_branch
    # make commits that would separate top_branch from master
    echo "q" > q.txt && git add q.txt && git commit -m "_q"
    echo "u" > u.txt && git add u.txt && git commit -m "_u"
    echo "v" > v.txt && git add v.txt && git commit -m "_v"
    # now make a commit that master will also have
    # the topbase will detect this as the fork point
    echo "a" > a.txt && git add a.txt && git commit -m "_a"
    # make the commit(s) that will actually be rebased:
    echo "x" > x.txt && git add x.txt && git commit -m "_x"

    git checkout master
    # make a commit that has the same blob(s) as the top_branch,
    # so it can be topbased on top of this commit
    echo "a" > a.txt && git add a.txt && git commit -m "_a"

    git checkout top_branch
    git_log_before_topbase="$(git log --oneline)"
    # topbase current branch onto master
    run mgt topbase master
    git_log_after_topbase="$(git log --oneline)"
    echo "$output"
    echo "git log before:"
    echo "$git_log_before_topbase"
    echo "git log after:"
    echo "$git_log_after_topbase"
    current_branch="$(git branch --show-current)"
    [[ "$current_branch" == "top_branch" ]]
    [[ "$git_log_after_topbase" != "$git_log_before_topbase" ]]
    [[ $status == 0 ]]
    [[ "$git_log_after_topbase" == *"_a"* ]]
    [[ "$git_log_after_topbase" == *"_x"* ]]
    # because topbase calculates the fork point differently, anything prior to the most recent
    # common blob (commit _a) will not be included
    [[ "$git_log_after_topbase" != *"_q"* ]]
}

@test 'can optionally specify a top branch' {
    git checkout -b top_branch
    # make commits that would separate top_branch from master
    echo "q" > q.txt && git add q.txt && git commit -m "_q"
    echo "u" > u.txt && git add u.txt && git commit -m "_u"
    echo "v" > v.txt && git add v.txt && git commit -m "_v"
    # now make a commit that master will also have
    # the topbase will detect this as the fork point
    echo "a" > a.txt && git add a.txt && git commit -m "_a"
    # make the commit(s) that will actually be rebased:
    echo "x" > x.txt && git add x.txt && git commit -m "_x"

    git checkout master
    # make a commit that has the same blob(s) as the top_branch,
    # so it can be topbased on top of this commit
    echo "a" > a.txt && git add a.txt && git commit -m "_a"

    git checkout top_branch
    git_log_before_topbase="$(git log --oneline)"
    git checkout master
    master_log_before_topbase="$(git log --oneline)"
    [[ "$(git branch --show-current)" == "master" ]]

    # topbase top_branch onto master, this will move us from master to top_branch
    echo "$(git status)"
    run mgt topbase master top_branch
    echo "$output"
    echo "---"
    echo "$(git status)"
    current_branch="$(git branch --show-current)"
    [[ "$current_branch" == "top_branch" ]]
    # master's history should not have changed
    [[ "$(git log master --oneline)" == "$master_log_before_topbase" ]]
    git_log_after_topbase="$(git log --oneline)"
    echo "git log before:"
    echo "$git_log_before_topbase"
    echo "git log after:"
    echo "$git_log_after_topbase"
    [[ "$git_log_after_topbase" != "$git_log_before_topbase" ]]
    [[ $status == 0 ]]
    [[ "$git_log_after_topbase" == *"_a"* ]]
    [[ "$git_log_after_topbase" == *"_x"* ]]
    # because topbase calculates the fork point differently, anything prior to the most recent
    # common blob (commit _a) will not be included
    [[ "$git_log_after_topbase" != *"_q"* ]]
}

@test 'doesnt include merge commits by default' {
    git checkout -b top_branch
    # make commits that would separate top_branch from master
    echo "q" > q.txt && git add q.txt && git commit -m "_q"
    echo "u" > u.txt && git add u.txt && git commit -m "_u"
    echo "v" > v.txt && git add v.txt && git commit -m "_v"
    # now make a commit that master will also have
    # the topbase will detect this as the fork point
    echo "a" > a.txt && git add a.txt && git commit -m "_a"
    # make the commit(s) that will actually be rebased:
    echo "x" > x.txt && git add x.txt && git commit -m "_x"
    git checkout -b merged_branch
    echo "y" > y.txt && git add y.txt && git commit -m "_y"
    git checkout top_branch
    git merge --no-edit --no-ff merged_branch
    echo "merge log?"
    echo "$(git log --oneline)"
    echo "z" > z.txt && git add z.txt && git commit -m "_z"

    git checkout master
    # make a commit that has the same blob(s) as the top_branch,
    # so it can be topbased on top of this commit
    echo "a" > a.txt && git add a.txt && git commit -m "_a"

    git checkout top_branch
    git_log_before_topbase="$(git log --oneline)"
    # topbase current branch onto master
    run mgt topbase master
    git_log_after_topbase="$(git log --oneline)"
    echo "$output"
    echo "git log before:"
    echo "$git_log_before_topbase"
    echo "git log after:"
    echo "$git_log_after_topbase"
    current_branch="$(git branch --show-current)"
    [[ "$current_branch" == "top_branch" ]]
    [[ "$git_log_after_topbase" != "$git_log_before_topbase" ]]
    [[ $status == 0 ]]
    [[ "$git_log_after_topbase" == *"_a"* ]]
    [[ "$git_log_after_topbase" == *"_x"* ]]
    [[ "$git_log_after_topbase" == *"_y"* ]]
    [[ "$git_log_after_topbase" == *"_z"* ]]
    # because topbase calculates the fork point differently, anything prior to the most recent
    # common blob (commit _a) will not be included
    [[ "$git_log_after_topbase" != *"_q"* ]]
    # it shouldn't contain the merge commit
    # (merge commit auto generates "Merge branch 'merged_branch' into top_branch")
    [[ "$git_log_after_topbase" != *"merged_branch"* ]]
}

@test 'dry-run should not modify anything' {
    git checkout -b top_branch
    # make commits that would separate top_branch from master
    echo "q" > q.txt && git add q.txt && git commit -m "_q"
    echo "u" > u.txt && git add u.txt && git commit -m "_u"
    echo "v" > v.txt && git add v.txt && git commit -m "_v"
    # now make a commit that master will also have
    # the topbase will detect this as the fork point
    echo "a" > a.txt && git add a.txt && git commit -m "_a"
    # make the commit(s) that will actually be rebased:
    echo "x" > x.txt && git add x.txt && git commit -m "_x"

    git checkout master
    # make a commit that has the same blob(s) as the top_branch,
    # so it can be topbased on top of this commit
    echo "a" > a.txt && git add a.txt && git commit -m "_a"

    git checkout top_branch
    git_log_before_topbase="$(git log --oneline)"
    # topbase dry run. dont actually change anything
    run mgt topbase master --dry-run
    echo "$output"
    git_log_after_topbase="$(git log --oneline)"
    echo "git log before:"
    echo "$git_log_before_topbase"
    echo "git log after:"
    echo "$git_log_after_topbase"
    [[ "$git_log_after_topbase" == "$git_log_before_topbase" ]]
    [[ $status == 0 ]]
}

@test 'can detect delete commits' {
    # this test is whether or not mgt can
    # detect a delete commit as a topbase fork point,
    git checkout -b top_branch
    # make commits that would separate top_branch from master
    echo "q" > q.txt && git add q.txt && git commit -m "_q"
    echo "u" > u.txt && git add u.txt && git commit -m "_u"
    echo "v" > v.txt && git add v.txt && git commit -m "_v"
    # now make a commit that master will also have
    # the topbase will detect this as the fork point
    echo "a" > a.txt && git add a.txt && git commit -m "_a"
    # make the commit(s) that will actually be rebased:
    rm a.txt && git add a.txt && git commit -m "DEL_A"

    git checkout master
    echo "a" > a.txt && git add a.txt && git commit -m "_a"
    # simulate master already having that delete:
    rm a.txt && git add a.txt && git commit -m "MASTER_REM_A"

    git checkout top_branch
    git_log_before_topbase="$(git log --oneline)"
    # topbase current branch onto master
    run mgt topbase master
    git_log_after_topbase="$(git log --oneline)"
    echo "$output"
    echo "git log before:"
    echo "$git_log_before_topbase"
    echo "git log after:"
    echo "$git_log_after_topbase"
    current_branch="$(git branch --show-current)"

    # the behavior is to go back to base branch
    # if nothing to topbase... for some reason...
    # maybe change this later. too lazy for now
    [[ "$current_branch" == "master" ]]
    [[ "$git_log_after_topbase" != "$git_log_before_topbase" ]]
    [[ $status == 0 ]]
    [[ "$git_log_after_topbase" == *"_a"* ]]
    [[ "$git_log_after_topbase" != *"DEL_A"* ]]
    # because topbase calculates the fork point differently, anything prior to the most recent
    # common blob (commit _a) will not be included
    [[ "$git_log_after_topbase" != *"_q"* ]]
}

@test 'A merge commit can be a fork point for topbase' {
    # this test is whether or not mgt can
    # detect a merge commit as a topbase fork point,
    # especially if the merge commit has a conflict resolved:
    git checkout -b top_branch
    # make commits that would separate top_branch from master
    echo "q" > q.txt && git add q.txt && git commit -m "_q"
    echo "u" > u.txt && git add u.txt && git commit -m "_u"
    echo "v" > v.txt && git add v.txt && git commit -m "_v"
    git checkout -b top-left
    echo "x1" > x1.txt && git add x1.txt && git commit -m "x1"
    git checkout -
    echo "x2" > x1.txt && git add x1.txt && git commit -m "x2"
    run git merge --no-ff top-left --no-commit
    echo "resolved" > x1.txt && git add x1.txt
    git commit -m "mergex1x2"
    # the state of x1, x2 in the merge commit should be
    # that they both exist, and contain "x1", and "x2"
    # respectively
    # so that was the fork point, now lets make a commit that will
    # be added to our main branch:
    echo "a" > a.txt && git add a.txt && git commit -m "_a"
    git log --raw --oneline -m --no-decorate

    git checkout master
    # simulate a commit that should be the same as the mergex1x2:
    echo "resolved" > x1.txt && git add x1.txt
    git commit -m "master-x1x2"

    git checkout top_branch
    echo "git log before:"
    git log --oneline --graph
    git_log_before_topbase="$(git log --oneline)"
    # topbase current branch onto master
    run mgt topbase master --verbose
    echo "git log after:"
    git log --oneline --graph
    git_log_after_topbase="$(git log --oneline)"
    echo "$output"
    current_branch="$(git branch --show-current)"

    [[ "$git_log_after_topbase" != "$git_log_before_topbase" ]]
    [[ $status == 0 ]]
    # the commit that was rebased from top branch onto master:
    [[ "$git_log_after_topbase" == *"_a"* ]]
    # anything before the fork point should not be included:
    [[ "$git_log_after_topbase" != *"_q"* ]]
}
