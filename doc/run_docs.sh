#!/usr/bin/env bash

# usage:
# must be run from repository root.
# ./doc/run_docs.sh

# to view the output:
# man ./dist/git-split.1.gz
# or <browser> ./dist/git-split.html
output_man_file() {
    source ./lib/constants.bsc
    source ./doc/build_manpages.sh
}

man_file_text=$(output_man_file)

echo "$man_file_text" > ./dist/git-split.1
cat ./dist/git-split.1 | groff -mandoc -Thtml > ./dist/git-split.html
gzip ./dist/git-split.1 -f
