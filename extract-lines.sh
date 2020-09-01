#!/usr/bin/env bash

file="$1"
search_for_str="$2"

if [[ ! -f $file ]]; then
    echo "Must provide a file to extract from"
    exit 1
fi

if [[ -z $search_for_str ]]; then
    echo "Must provide a string to search for"
    exit 1
fi

while read line; do
    if [[ $line == "$search_for_str"* ]]; then
        # replace first occurrence of $search_for_str
        echo "${line/$search_for_str/}"
    fi
done < "$file"
