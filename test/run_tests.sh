#!/usr/bin/env bash

source_combine git-split.bsc > test/git-split
chmod +x test/git-split
bats test/
