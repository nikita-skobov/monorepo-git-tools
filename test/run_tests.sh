#!/usr/bin/env bash

cargo build --release
# this should output to ./target/release/my-git-tools
PROGRAM_PATH="./target/release/my-git-tools"

run_all_tests() {
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

if [[ ! -f $PROGRAM_PATH ]]; then
    echo "Failed to find output program to run tests with: $PROGRAM_PATH"
    exit 1
fi

run_all_tests

# # prevent running tests that involve remote access:
# if [[ $1 == "-l" || $1 == "--local-only" ]]; then
    # mv test/split/end-to-end-remote.bats test/tmpe2e.txt
    # run_all_tests
    # mv test/tmpe2e.txt test/split/end-to-end-remote.bats
# else
    # run_all_tests
# fi
