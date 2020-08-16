#!/usr/bin/env bash

verbose=""
if [[ "$@" == *"-v"* ]]; then
    verbose="true"
fi

cargo_output=$(cargo build --release 2>&1)
if [[ $? != "0" ]]; then
    echo "$cargo_output"
    echo ""
    echo "Failed to run cargo build"
    echo "Tests will not run"
    exit 1
fi
# this should output to ./target/release/my-git-tools
PROGRAM_PATH="./target/release/my-git-tools"

if [[ ! -f $PROGRAM_PATH ]]; then
    echo "Failed to find output program to run tests with: $PROGRAM_PATH"
    exit 1
fi


run_unit_tests() {
    cargo test 2>tempfile.txt
    if [[ $? != "0" ]]; then
        echo "Unit tests not successful:"
        if [[ ! -z $verbose ]]; then
            echo "$(<tempfile.txt)"
        else
            echo "re-run this test script with -v to see detailed output"
        fi
        echo "Next tests will not run"
        rm tempfile.txt
        exit 1
    fi
}

run_end_to_end_tests() {
    echo "GENERAL PROGRAM:"
    PROGRAM_PATH="$PROGRAM_PATH" bats test/general
    echo ""
    # echo "HELPER FUNCTIONS:"
    # bats test/helpers
    # echo ""
    # echo "GIT SPLIT:"
    # bats test/splitin
    # echo ""
    # echo "GIT TOPBASE:"
    # bats test/topbase
    # echo ""
}

run_all_tests() {
    echo "UNIT TESTS:"
    run_unit_tests
    echo "END-TO-END TESTS:"
    run_end_to_end_tests
}

run_all_tests
