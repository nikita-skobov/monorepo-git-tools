#!/usr/bin/env bash

cat doc/repo_file.template
./extract-lines.sh src/repo_file.rs "///"
