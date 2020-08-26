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


function teardown() {
    cd $BATS_TMPDIR
    if [[ -d test_remote_repo ]]; then
        rm -rf test_remote_repo
    fi
    if [[ -d test_remote_repo2 ]]; then
        rm -rf test_remote_repo2
    fi
}

@test 'should show usage when using -h --help or help' {
    run $PROGRAM_PATH help
    echo "$output"
    [[ $output == *"USAGE"* ]]
    [[ $output == *"help"* ]]
    [[ $output == *"version"* ]]

    run $PROGRAM_PATH --help
    echo "$output"
    [[ $output == *"USAGE"* ]]
    [[ $output == *"help"* ]]
    [[ $output == *"version"* ]]

    run $PROGRAM_PATH -h
    echo "$output"
    [[ $output == *"USAGE"* ]]
    [[ $output == *"help"* ]]
    [[ $output == *"version"* ]]
}

@test 'should show current cargo.toml version' {
    cargo_txt=$(<./Cargo.toml)
    version_line=""

    last_ifs=$IFS
    IFS=$'\n'
    for line in $cargo_txt; do
        echo "line: $line"
        if [[ $line == *"version ="* ]]; then
            version_line="$line"
            break
        fi
    done
    IFS=$last_ifs

    version_str="$(echo $version_line | cut -d'"' -f 2)"

    run $PROGRAM_PATH --version
    [[ $output == *"$version_str"* ]]
}

@test 'can use dry run as -d or --dry-run' {
    make_temp_repo test_remote_repo
    make_temp_repo test_remote_repo2
    cd $BATS_TMPDIR/test_remote_repo
    run $PROGRAM_PATH split-in-as "$BATS_TMPDIR/test_remote_repo2" --as somesubdir --dry-run
    echo "$output"

    # some pattern matching test to make sure it outputs git commands in the dry run output
    [[ $output == *"git pull /tmp"* ]]
    [[ $status == "0" ]]

    # now do the same but with -d
    run $PROGRAM_PATH split-in-as "$BATS_TMPDIR/test_remote_repo2" --as somesubdir -d
    echo "$output"

    # some pattern matching test to make sure it outputs git commands in the dry run output
    [[ $output == *"git pull /tmp"* ]]
    [[ $status == "0" ]]
}
