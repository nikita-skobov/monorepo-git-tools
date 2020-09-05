#!/usr/bin/env bash

# this is meant to run in a pipeline.
# if any command fails, we want the script to exit with error
set -e

git clone https://github.com/newren/git-filter-repo
if [[ $@ == *"windows"* ]]; then
  # if on windows, replace their python3 shebang with python.
  # the github windows os image has python installed, but there is no python3 alias
  sed -i 's/#!\/usr\/bin\/env\ python3/#!\/usr\/bin\/env\ python/g' git-filter-repo/git-filter-repo
  cp git-filter-repo/git-filter-repo "$(git --exec-path)/git-filter-repo"
else
  sudo cp git-filter-repo/git-filter-repo $(git --exec-path)/git-filter-repo
fi

# verify it works, and then delete the rest of the source
git filter-repo --version
rm -rf git-filter-repo/
