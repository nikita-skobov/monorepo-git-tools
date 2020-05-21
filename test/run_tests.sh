#!/usr/bin/env bash

source_combine git-split.bsc > dist/git-split_t
chmod +x dist/git-split_t
bats test/
