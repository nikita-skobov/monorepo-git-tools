#!/usr/bin/env bash

# usage:
# must be run from repository root.
# ./doc/run_docs.sh

# to view the output:
# man ./dist/git-split.1.gz
# or <browser> ./dist/git-split.html
output_man_file() {
    source ./lib/constants/$1.bsc
    source ./lib/constants/repo_file.bsc

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

    # only render the repo file docs section
    # for certain tools.
    if [[ $1 == "git_split" ]]; then
        repo_file_custom_section=(
            "About the repo file"
            "I created these tools with the intention of defining repo_files that contain information on how to split out/in local repositories back and forth from remote repositories. A repo_file is just a shell script that contains some variables. It is sourced by commands in this repository, and the variables that it sources are used to do the splitting out/in."
            ""
            "The following is a list of all valid variables you can use in your repo file."
            ""
            "The format is: <variable_name> - (type) - [required|optional|conditional]"
            ""
            "If the requiredness is 'conditional', read the paragraph under that variable name for an explanation"
            ""
            ""
            ""
        )
        for i in "${rfv_valid_variable_names[@]}"; do
            local -n array="rfv_$i"
            # get a reference to the array
            # i: variable name
            # array[0]: type, [1]: opt/reqd, [2]: doc string
            repo_file_custom_section+=("\fI$i\fR \- (${array[0]}) \- ${array[1]}")
            repo_file_custom_section+=("${array[2]}")
        done
        doc_custom_sections+=("repo_file_custom_section")
    fi

    source ./doc/build_manpages.sh
}

programs=(
    "git_split"
    "git_topbase"
)

for _name in ${programs[@]}; do
    man_file_text=$(output_man_file $_name)
    name_with_dash="${_name//_/-}"

    echo "$man_file_text" > ./dist/$name_with_dash.1
    cat ./dist/$name_with_dash.1 | groff -mandoc -Thtml > ./dist/$name_with_dash.html
    gzip ./dist/$name_with_dash.1 -f
done
