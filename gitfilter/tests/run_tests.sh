#!/usr/bin/env bash

# thanks stackoverflow: https://stackoverflow.com/a/4774063
SCRIPTPATH="$( cd "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

path_to_react_repo_root="$1"
path_to_gitfiltercli="$2"

if [[ -z $GITFILTERCLI ]]; then
    GITFILTERCLI="$path_to_gitfiltercli"
fi
if [[ -z $PATHTOREACTROOT ]]; then
    PATHTOREACTROOT="$path_to_react_repo_root"
fi

# only try to source if user didnt provide anything
if [[ -z $PATHTOREACTROOT && -z $GITFILTERCLI ]]; then
    PATH_TO_TEST_VARS="$SCRIPTPATH/test_vars.sh"
    source $PATH_TO_TEST_VARS
fi

if [[ -z $PATHTOREACTROOT || -z $GITFILTERCLI ]]; then
    echo "Missing cli input. need path to react repo root, and path to git filter cli"
    exit 1
fi

echo "GITFILTERCLI: $GITFILTERCLI"
echo "PATHTOREACTROOT: $PATHTOREACTROOT"

bats "$SCRIPTPATH/"
