#!/usr/bin/env bash

# usage:
# must be run from repository root.
# ./doc/run_docs.sh

# to view the output:
# man ./dist/git-split.1.gz
# or <browser> ./dist/git-split.html
output_man_file() {
    source ./lib/constants.bsc

    # prepend the arrays defined in constants
    # because the man page is slightly
    # different than the --help text
    local temp_doc_global_options=("Global options:")
    for i in "${doc_global_options[@]}"; do
        temp_doc_global_options+=("$i")
    done
    doc_global_options=()
    for i in "${temp_doc_global_options[@]}"; do
        doc_global_options+=("$i")
    done
    local temp_doc_custom_sections=("doc_global_options")
    for i in "${doc_custom_sections[@]}"; do
        temp_doc_custom_sections+=("$i")
    done
    doc_custom_sections=()
    for i in "${temp_doc_custom_sections[@]}"; do
        doc_custom_sections+=("$i")
    done

    source ./doc/build_manpages.sh
}

man_file_text=$(output_man_file)

echo "$man_file_text" > ./dist/git-split.1
cat ./dist/git-split.1 | groff -mandoc -Thtml > ./dist/git-split.html
gzip ./dist/git-split.1 -f
