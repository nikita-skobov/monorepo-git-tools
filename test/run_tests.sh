#!/usr/bin/env bash

source_combine git-split.bsc > test/split/git-split
source_combine git-topbase.bsc > test/topbase/git-topbase
chmod +x test/split/git-split
chmod +x test/topbase/git-topbase

# prevent running tests that involve remote access:
if [[ $1 == "-l" || $1 == "--local-only" ]]; then
    mv test/split/end-to-end-remote.bats test/tmpe2e.txt
    bats test/split
    mv test/tmpe2e.txt test/split/end-to-end-remote.bats
else
    bats test/split
fi

