#!/usr/bin/env bash

cat doc/repo_file.template
# temporarily remove this, as the repo_file format has
# changed, and I removed the doc comments.
# TODO: add doc comments back to the repo file module
# ./extract-lines.sh src/repo_file.rs "///"
