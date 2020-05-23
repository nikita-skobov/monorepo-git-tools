#!/usr/bin/env bash

# usage:
# ./doc/run_docs.sh > git-split.txt
# cat ./git-split.txt | groff -mandoc -Thtml > git-split.html
output_man_file() {
    source ./lib/constants.bsc
    source ./doc/build_manpages.sh
}

man_file_text=$(output_man_file)

echo "$man_file_text" > ./dist/git-split.1
cat ./dist/git-split.1 | groff -mandoc -Thtml > ./dist/git-split.html
gzip ./dist/git-split.1

