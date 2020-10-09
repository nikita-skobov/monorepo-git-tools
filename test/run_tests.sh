#!/usr/bin/env bash

should_run_git_unit_tests=""
verbose=""
if [[ "$@" == *"-v"* ]]; then
    verbose="true"
fi
if [[ "$@" == *"-g"* ]]; then
    should_run_git_unit_tests="true"
fi


build_program() {
    cargo_output=$(cargo build --release 2>&1)
    if [[ $? != "0" ]]; then
        echo "$cargo_output"
        echo ""
        echo "Failed to run cargo build"
        echo "Tests will not run"
        exit 1
    fi
    # this should output to ./target/release/my-git-tools
    PROGRAM_PATH="./target/release/mgt"
    PROGRAM_PATH="$(realpath $PROGRAM_PATH)"

    if [[ ! -f $PROGRAM_PATH ]]; then
        echo "Failed to find output program to run tests with: $PROGRAM_PATH"
        exit 1
    fi
}

run_unit_tests() {
    if [[ ! -z $should_run_git_unit_tests ]]; then
        cargo test --features gittests 2>tempfile.txt
    else
        cargo test 2>tempfile.txt
    fi
    if [[ $? != "0" ]]; then
        echo "Unit tests not successful:"
        if [[ ! -z $verbose ]]; then
            echo "$(<tempfile.txt)"
        else
            echo "re-run this test script with -v to see detailed output"
        fi
        echo "Next tests will not run"
        exit 1
    fi
    rm tempfile.txt
}

run_end_to_end_tests() {
    echo "GENERAL PROGRAM:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/general
    echo ""
    echo "SPLIT-OUT:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/splitout
    echo ""
    echo "SPLIT-IN:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/splitin
    echo ""
    echo "SPLIT-IN-AS:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/splitinas
    echo ""
    echo "GIT TOPBASE:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/topbase
    echo ""
    echo "SPLIT-OUT-AS:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/splitoutas
    echo ""
    echo "CHECK:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/check
    echo ""
}

run_all_tests() {
    echo "BUILDING..."
    build_program
    echo "UNIT TESTS:"
    run_unit_tests
    echo "END-TO-END TESTS:"
    run_end_to_end_tests
}

run_all_tests
