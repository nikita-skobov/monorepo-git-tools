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

# TODO: maybe even let each test run in parallel
# via randomly generated folders for each test case?
function random_folder_name() {
    chars=abcdef01234567
    for i in {1..16} ; do
        echo -n "${chars:RANDOM%${#chars}:1}"
    done
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
    test_folder="$BATS_TMPDIR/splitin"
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

function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
    fi
    cd ..
    if [[ -d splitin ]]; then
        rm -rf splitin/
    fi
}

@test 'can split in a remote_repo uri' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
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

@test 'should not run if user has modified files' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"lib/\" \" \"
    )
    "
    echo "$repo_file_contents" > repo_file.sh

    # create a modified file
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    # now modify it so that it shows up to git as modified but unstaged
    echo "abcd" > abc.txt

    echo "$(git status)"
    echo "$(find . -not -path '*/\.*')"

    git_log_before="$(git log --oneline)"
    # this should exit with a warning that you have modified files
    # and we should not run, otherwise your changes might be lost
    run $PROGRAM_PATH split-in repo_file.sh --verbose -o newbranch1
    git_log_after="$(git log --oneline)"
    echo "$output"
    echo "$(git status)"
    echo "$(find . -not -path '*/\.*')"
    [[ $status != "0" ]]
    [[ "$(git branch --show-current)" == "master" ]]
    [[ "$git_log_before" == "$git_log_after" ]]
    [[ $output == *"modified changes"* ]]
}

@test 'can split in to a specific output branch' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
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

@test 'can split in to a specific output branch (overrides repo_name)' {
    repo_file_contents="
    repo_name=\"abcd\"
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"this/path/will/be/created/\" \" \"
    )
    "

    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH split-in repo_file.sh --verbose -o newbranch1
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == *"newbranch1"* ]]
}

@test 'can split in a remote_repo with a specific remote_branch' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
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

@test 'can include only parts of remote repos' {
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
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"locallib/\" \"lib/\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"

    # since we excluded lib, it shouldnt be there
    # but rootfile1 should
    [[ -d locallib ]]

    [[ -f locallib/libfile1.txt ]]
    [[ -f locallib/libfile2.txt ]]
    [[ ! -f libfile1.txt ]]
    [[ ! -f rootfile1.txt ]]
}

@test 'works when sources have spaces in them' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p "my lib"
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > "my lib/libfile1.txt"
    echo "libfile2.txt" > "my lib/libfile2.txt"
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"locallib/\" \"my lib/\"
    )
    "

    echo "$repo_file_contents" > repo_file.sh
    run $PROGRAM_PATH split-in repo_file.sh --verbose
    echo "$output"
    [[ $status == "0" ]]
    echo "$(find . -not -path '*/\.*')"

    # since we excluded lib, it shouldnt be there
    # but rootfile1 should
    [[ -d locallib ]]
    [[ ! -d "my lib/" ]]

    [[ -f locallib/libfile1.txt ]]
    [[ -f locallib/libfile2.txt ]]
    [[ ! -f libfile1.txt ]]
    [[ ! -f rootfile1.txt ]]
}

@test 'can split in only n latest commits' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"

    echo "1a" > 1a.txt && git add 1a.txt && git commit -m "1a"
    echo "2b" > 2b.txt && git add 2b.txt && git commit -m "2b"
    echo "3c" > 3c.txt && git add 3c.txt && git commit -m "3c"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(\"lib/\" \" \")
    "
    echo "$repo_file_contents" > repo_file.sh

    # we only want the latest 2 commits
    run $PROGRAM_PATH split-in repo_file.sh --verbose --num-commits 2
    echo "$output"
    [[ $status == "0" ]]
    git_branch=$(git branch --show-current)
    git_log_now=$(git log --oneline)
    num_commits="$(git log --oneline | wc -l)"
    echo "$git_log_now"
    echo "$git_branch"
    [[ $git_branch == "doesnt_matter" ]]
    # because n was 2, there should only be the top 2 commits
    [[ $num_commits == "2" ]]
    [[ $git_log_now != *"1a"* ]]
    [[ $git_log_now == *"2b"* ]]
    [[ $git_log_now == *"3c"* ]]
}

@test 'properly handles nested folder renames/moves' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    echo "srcfile1.txt" > srcfile1.txt
    echo "srcfile2.txt" > lib/srcfile2.txt
    git add .
    git commit -m "adds files"
    echo "repo2 dir:"
    echo "$(find . -type f -not -path '*/\.*')"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"lib/src/srcfile1.txt\" \"srcfile1.txt\"
        \"lib/src/\" \"lib/\"
        \"lib/\" \" \"
    )
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    echo "$output"
    echo "local repo dir after split:"
    echo "$(find . -type f -not -path '*/\.*')"

    [[ $status == "0" ]]

    [[ -d lib ]]
    [[ ! -d lib/lib ]]
    [[ -d lib/src ]]
    [[ -f lib/src/libfile1.txt ]]
    [[ -f lib/src/libfile2.txt ]]
    [[ -f lib/src/srcfile1.txt ]]
    [[ -f lib/src/srcfile2.txt ]]
    [[ -f lib/rootfile1.txt ]]
}

@test 'can include without renaming' {
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
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"lib/libfile1.txt\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"


    [[ -f lib/libfile1.txt ]]
    [[ ! -f lib/libfile2.txt ]]
    [[ ! -f rootfile1.txt ]]
    [[ ! -f test_remote_repo2.txt ]]
}

@test 'can include folders without renaming' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    mkdir -p src
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    echo "srcfile1.txt" > src/srcfile1.txt
    echo "srcfile2.txt" > src/srcfile2.txt
    git add lib
    git commit -m "adds 2 lib files and 1 root file"
    git add src
    git commit -m "adds src"
    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=(\"src/\" \"lib\")
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"


    [[ -f lib/libfile1.txt ]]
    [[ -f lib/libfile2.txt ]]
    [[ -f src/srcfile1.txt ]]
    [[ -f src/srcfile2.txt ]]
    [[ ! -f rootfile1.txt ]]
    [[ ! -f test_remote_repo2.txt ]]
}

@test 'can include a folder and exclude a subfolder' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    mkdir -p lib/test
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    echo "testfile1.txt" > lib/test/testfile1.txt
    echo "testfile2.txt" > lib/test/testfile2.txt
    git add .
    git commit -m "adds tests"

    cd "$curr_dir"

    repo_file_contents="
    repo_name=\"doesnt_matter\"
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"lib/\"
    exclude=\"lib/test/\"
    "

    echo "$repo_file_contents" > repo_file.sh
    echo "$(git split in repo_file.sh --dry-run)"

    run $PROGRAM_PATH split-in repo_file.sh --verbose
    [[ $status == "0" ]]
    echo "$output"
    echo "$(find . -not -path '*/\.*')"


    [[ -f lib/libfile1.txt ]]
    [[ -f lib/libfile2.txt ]]
    [[ ! -d lib/test ]]
    [[ ! -f lib/test/testfile1.txt ]]
    [[ ! -f lib/test/testfile2.txt ]]
    [[ ! -f test_remote_repo2.txt ]]
    [[ ! -f rootfile1.txt ]]
}

# rebase onto means the changes stay on the new branch, but
# it uses the original branch as the upstream branch to compare with
@test 'can optionally rebase the new branch onto original branch' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    master_commits="$(git log --oneline | wc -l)"
    made_commits=0
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    ((made_commits += 1))
    cd "$curr_dir"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"lib/\"
    "

    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH split-in repo_file.sh -r --verbose
    echo "$output"
    [[ $status == "0" ]]
    [[ "$(git branch --show-current)" == "test_remote_repo2" ]]
    output_commits="$(git log --oneline | wc -l)"
    echo ""
    echo "$(git log --oneline)"
    echo ""

    # we test that the number of commits is now the number that we made in master
    # plus the number we made in the new branch that got filtered.
    echo "output_commits ($output_commits) =?= master_commits + made_commits ($((master_commits + made_commits)))"
    [[ "$output_commits" == "$((master_commits + made_commits))" ]]

    [[ -f lib/libfile1.txt ]]
    [[ -f lib/libfile2.txt ]]
    [[ ! -f rootfile1.txt ]]
    # since we specified to rebase, this file should exist
    # because it existed in master, and we rebased our new branch on top of master
    [[ -f test_remote_repo.txt ]]
}

@test '--rebase should not say success if there were rebase merge conflicts' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "123" > abc.txt && git add abc.txt && git commit -m "abc-123"
    cd "$curr_dir"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(\"lib/\" \" \")
    "
    echo "$repo_file_contents" > repo_file.sh

    mkdir -p lib
    # this is where it aligns:
    echo "abc" > lib/abc.txt && git add lib/abc.txt && git commit -m "abc"
    # this is the conflict:
    echo "conflicthere" > lib/abc.txt && git add lib/abc.txt && git commit -m "conflict"

    run $PROGRAM_PATH split-in repo_file.sh --rebase --verbose
    echo "$output"
    echo "$(git status)"
    [[ $status != "0" ]]
    [[ "$output" != *"Success!"* ]]
    [[ "$(git status)" == *"rebase in progress"* ]]
}

@test '--topbase should not say success if there were rebase merge conflicts' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "123" > abc.txt && git add abc.txt && git commit -m "abc-123"
    cd "$curr_dir"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(\"lib/\" \" \")
    "
    echo "$repo_file_contents" > repo_file.sh

    mkdir -p lib
    # this is where it aligns:
    echo "abc" > lib/abc.txt && git add lib/abc.txt && git commit -m "abc"
    # this is the conflict:
    echo "conflicthere" > lib/abc.txt && git add lib/abc.txt && git commit -m "conflict"

    run $PROGRAM_PATH split-in repo_file.sh --topbase --verbose
    echo "$output"
    echo "$(git status)"
    [[ $status != "0" ]]
    [[ "$output" != *"Success!"* ]]
    [[ "$(git status)" == *"rebase in progress"* ]]
}

@test 'can specify a branch to topbase from' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    mkdir -p lib
    echo "abc" > abc.txt && git add abc.txt && git commit -m "abc"
    echo "123" > abc.txt && git add abc.txt && git commit -m "abc-123"
    git checkout -b b456
    echo "456" > abc.txt && git add abc.txt && git commit -m "commit_456"
    git checkout -
    cd "$curr_dir"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(\"lib/\" \" \")
    "
    echo "$repo_file_contents" > repo_file.sh

    mkdir -p lib
    # this is where it aligns:
    echo "abc" > lib/abc.txt && git add lib/abc.txt && git commit -m "abc"

    run $PROGRAM_PATH split-in repo_file.sh --topbase b456 --verbose
    echo "$output"
    echo "$(git status)"
    [[ $status == "0" ]]
    [[ "$output" == *"Success!"* ]]
    [[ "$(git log --oneline)" == *"commit_456"* ]]
}

@test 'should not be able to use --topbase with --rebase' {
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(\"lib/\" \" \")
    "
    echo "$repo_file_contents" > repo_file.sh

    run $PROGRAM_PATH split-in repo_file.sh --rebase --topbase --verbose
    echo "$output"
    [[ $status != "0" ]]
    [[ "$output" != *"Success!"* ]]
    [[ "$output" == *"Cannot use both"* ]]
}

@test '--topbase should not say success if there were rebase merge conflicts (if take all remote)' {
    # save current dir to cd back to later
    curr_dir="$PWD"
    # setup the test remote repo:
    cd "$BATS_TMPDIR/test_remote_repo2"
    made_commits=0
    mkdir -p lib
    echo "rootfile1.txt" > rootfile1.txt
    echo "libfile1.txt" > lib/libfile1.txt
    echo "libfile2.txt" > lib/libfile2.txt
    git add .
    git commit -m "adds 2 lib files and 1 root file"
    cd "$curr_dir"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"lib/\"
    "

    echo "$repo_file_contents" > repo_file.sh
    mkdir -p lib
    echo "conflict" > lib/libfile1.txt && git add lib/libfile1.txt && git commit -m "conflict"

    run $PROGRAM_PATH split-in repo_file.sh --topbase --verbose
    echo "$output"
    echo "$(git status)"
    [[ $status != "0" ]]
    [[ "$output" != *"Success!"* ]]
    [[ "$(git status)" == *"rebase in progress"* ]]
}

@test 'if topbase finds 0, it shouldnt rebase interactively' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    # for this test, we want to ensure that the remote repo has 2 commits
    # in its history after the rewrite:
    echo "doesntmatter" > file1.txt
    git add file1.txt && git commit -m "doesntmatter"
    # now this commit matters because it should have the same blob as the local repository will
    echo "file1" > file1.txt
    git add file1.txt && git commit -m "file1"

    # simulate the remote repo and the local repo being
    # up to date with each other (same blob as above)
    cd "$curr_dir"
    echo "file1" > file1.txt
    git add file1.txt && git commit -m "file1_local"

    echo "on master local:"
    echo "$(git log --oneline)"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"file1.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    git_log_before="$(git log --oneline)"
    run $PROGRAM_PATH split-in repo_file.sh -t --verbose
    echo "$output"
    echo "$(git log --oneline)"
    git_log_now="$(git log --oneline)"
    [[ $status == "0" ]]
    [[ $output == *"rebasing non-interactively"* ]]
    # it should still rebase because that will make the output
    # branch fast-forwardable
    [[ $git_log_now == $git_log_before ]]
}

@test 'if topbase finds 0 (AND remote only has one commit), it shouldnt rebase interactively' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "file1" > file1.txt
    git add file1.txt && git commit -m "file1"

    # simulate the remote repo and the local repo being
    # up to date with each other (same blob as above)
    cd "$curr_dir"
    echo "file1" > file1.txt
    git add file1.txt && git commit -m "file1_local"

    echo "on master local:"
    echo "$(git log --oneline)"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"file1.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    git_log_before="$(git log --oneline)"
    run $PROGRAM_PATH split-in repo_file.sh -t --verbose
    echo "$output"
    echo "$(git log --oneline)"
    git_log_now="$(git log --oneline)"
    [[ $status == "0" ]]
    [[ $output == *"rebasing non-interactively"* ]]
    # it should still rebase because that will make the output
    # branch fast-forwardable
    [[ $git_log_now == $git_log_before ]]
}

@test 'if topbase finds the entire history, it shouldnt rebase interactively' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"
    echo "file1" > file1.txt
    git add file1.txt && git commit -m "file1"
    cd "$curr_dir"

    echo "on master local:"
    echo "$(git log --oneline)"

    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include=\"file1.txt\"
    "
    echo "$repo_file_contents" > repo_file.sh
    git_log_before="$(git log --oneline)"
    run $PROGRAM_PATH split-in repo_file.sh -t --verbose
    echo "$output"
    echo "$(git log --oneline)"
    git_log_now="$(git log --oneline)"
    [[ $status == "0" ]]
    [[ $output == *"rebasing non-interactively"* ]]
}

@test 'can rename weird file and folder names' {
    curr_dir="$PWD"
    cd "$BATS_TMPDIR/test_remote_repo2"

    echo "a" > a.txt
    git add a.txt && git commit -m "a"
    echo "b" > "du[mbfile.txt"
    git add . && git commit -m "dumbfile"
    mkdir -p "dum'lib"
    echo "lib" > "dum'lib/lib.txt"
    git add . && git commit -m "dumblib"

    echo "$(find . -type f -not -path '*/\.*')"

    [[ -f a.txt ]]
    [[ -f "du[mbfile.txt" ]]
    [[ -d "dum'lib/" ]]

    cd "$curr_dir"

    # I originally wanted to also maybe support files with \ in
    # them, but i think it'd be too difficult due to multiple levels of
    # escaping, and also it being treated differently on windows vs linux
    repo_file_contents="
    remote_repo=\"..$SEP$test_remote_repo2\"
    include_as=(
        \"dumbfile.txt\" \"du[mbfile.txt\"
        \"spaghetti/\" \"dum'lib/\"
    )
    "
    echo "$repo_file_contents" > repo_file.sh
    echo "repo file contents:"
    cat repo_file.sh

    [[ ! -f "dumbfile.txt" ]]

    run $PROGRAM_PATH split-in repo_file.sh --verbose

    echo "$(find . -type f -not -path '*/\.*')"

    echo "$output"
    [[ $status == "0" ]]

    [[ ! -f a.txt ]]
    [[ -f "dumbfile.txt" ]]
    [[ -d spaghetti/ ]]
    [[ ! -d "dum'lib/" ]]
    [[ -f "spaghetti/lib.txt" ]]
}
