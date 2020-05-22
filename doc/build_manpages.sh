#!/usr/bin/env bash

# usage:
# this script relies on variables existing
# so make sure they exist in your shell
# prior to running

# # example of variables that need to be defined
# doc_program_name="git-split"
# doc_synopsis=(
#     "out <repo_file> [OPTIONS]"
#     "in <repo_file> [OPTIONS]"
# )
# # use: \fI YOUR TEXT HERE \fR
# # for italicized/underlined code snippets
# doc_commands=(
#     "out"
#     "this is the indented block for out \fIgit pull\fR"
#     "in"
#     "this is block for in!"
# )
# doc_author="Nikita Skobov"
# doc_author_email="skobo002@umn.edu"





append_synopsis_str () {
    if [[ ! -z $synopsis_str ]]; then
synopsis_str="$synopsis_str
.br
"
    fi
synopsis_str="$synopsis_str.B $doc_program_name
$1"
}

program_name_upper="${doc_program_name^^}"
month_year="$(date +'%b %Y')"


short_description="${doc_short_description:-"manual page for $doc_program_name"}"

if [[ ! -z $doc_description ]]; then
description_block=".SH DESCRIPTION
$doc_description"
else
    description_block=""
fi

for i in "${doc_synopsis[@]}"; do
    append_synopsis_str "$i"
done

command_block=".SH COMMANDS
"
is_command_name="true"
for i in "${doc_commands[@]}"; do
    if [[ $is_command_name == "true" ]]; then
command_block="$command_block.PP
$i
"
        is_command_name="false"
    else
command_block="$command_block.RS 4
$i
.RE
"
        is_command_name="true"
    fi
done

if [[ ! -z $synopsis_str ]]; then
synopsis_str=".SH SYNOPSIS
$synopsis_str"
fi



echo "
.TH \"$program_name_upper\" \"1\" \"$month_year\" \"$doc_program_name \" \"User Commands\"
.LG
.SH NAME
$doc_program_name \- $short_description
$synopsis_str
$description_block
$command_block
.SH AUTHOR
.PP
Written by $doc_author <$doc_author_email>
"
