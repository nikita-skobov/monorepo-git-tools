#!/usr/bin/env bash
# must be run from the root of this repository

./doc/gen_repo_file.sh > doc/repo_file.md
./doc/gen_readme.sh > doc/README.md
./doc/gen_split_out.sh > doc/split-out.md
./doc/gen_split_out_as.sh > doc/split-out-as.md
./doc/gen_split_in.sh > doc/split-in.md
./doc/gen_split_in_as.sh > doc/split-in-as.md
./doc/gen_topbase.sh > doc/topbase.md
./doc/gen_check.sh > doc/check.md
./doc/gen_verify_rf.sh > doc/verify-rf.md
