#!/usr/bin/env bash

# usage:
# ./doc/run_docs.sh > git-split.txt
# cat ./git-split.txt | groff -mandoc -Thtml > git-split.html

source ./lib/constants.bsc
source ./doc/build_manpages.sh
