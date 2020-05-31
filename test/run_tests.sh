#!/usr/bin/env bash

source_combine git-split.bsc > test/git-split
source_combine git-topbase.bsc > test/git-topbase
chmod +x test/git-topbase
chmod +x test/git-split

# prevent running tests that involve remote access:
if [[ $1 == "-l" || $1 == "--local-only" ]]; then
    mv test/end-to-end-remote.bats test/tmpe2e.txt
    bats test/
    mv test/tmpe2e.txt test/end-to-end-remote.bats
else
    bats test/
fi

