# mgt (monorepo-git-tools)

> Git tools that enable easy bidirectional sync between multiple repositories

## Table of contents

* [What does it do?](#what-does-it-do)
* [How does it work?](#how-does-it-work)
* [Prerequisites](#prerequisites)
* [Installation](#installation)
* [What is a "repo file"?](#what-is-a-repo-file)

## What does it do?

`mgt` has several features that make it easy to collaborate with
projects outside of your repository. It works by rewriting
your git history onto a temporary branch in a way such that the branch
can be easily merged into another repository.

Some features that `mgt` provides are:

- splitting your private repository into subrepositories that only contain files/folders that you want to share publically
- bidirectional sync between repositories via transient branches that are formatted to be rebased
- checking updates of multiple subrepositories (TODO)
- filtering out specific text/commits before sharing publically (TODO)

## How does it work?

`mgt` knows how to rewrite your history based on files contain information on how to rewrite the history.
These files are called `repo_file`s, and some common things that would
go in a `repo_file` are:

- including files/folders
- renaming files/folders
- excluding files/folders
- specifying repository URL(s) to use as the destination

When you run `mgt`, you typically provide a path to the `repo_file`, eg: `mgt split-out my_repo_file.txt`

## Prerequisites

These commands are just "porcelain" commands built on top of the real "plumbing" capabilities of [git-filter-repo](https://github.com/newren/git-filter-repo).

You must have `git-filter-repo` available on your path prior to using the tools in this repository.


## Installation

TODO

## What is a "repo file"?

I created these tools with the intention of defining `repo_file`s that contain information on how to split out/in local repositories back and forth from remote repositories. A `repo_file` is just a text file that contains some variables in a very limited bash syntax. 

Here is a commented `repo_file` that explains what some of the common variables do. For a full explanation, see the [repo file explanation section of the manual](https://htmlpreview.github.io/?https://github.com/nikita-skobov/git-monorepo-tools/blob/master/dist/git-split.html#ABOUT%20THE%20REPO%20FILE)

```sh
# used for: git pull $remote_repo when doing
# git split in
remote_repo="https://github.com/myname/myrepo"

# instead of pulling remote_repo from HEAD,
# it can pull from a specific branch instead
remote_branch="feature/X"

# used for the branch name that gets output.
# also, if `remote_repo` is NOT provided,
# and `username` IS PROVIDED, then `git split in`
# will use https://github.com/$username/$repo_name
repo_name="git-monorepo-tools"

# includes the source repository files/directories
# exactly as is, without changing the paths
include=(
    "doc/some_file.txt"
    "scripts/"
)

# another example of include:
# include can just be a string:
include="scripts/"

# includes the source files/folders into the destination files/folders
# ie: use this variable if you wish to rename paths
# so below, lib/cool-lib corresponds to an empty string
# this means that when you split out a repo, it will take everything from
# lib/cool-lib/ and put it in the root of the destination repo, ignoring
# all files/folders other than lib/cool-lib 
include_as=(
    "lib/cool-lib/" ""
)

# Another example of include_as:
# in this example we rename one of the lib files
# and we also move a directory to a different part of the
# destination
include_as=(
    "lib/get_arg.sh" "lib/get_arg.bsc"
    "repos/my_blog/" "lib/my_blog/"
)

# excludes the source files/folders from
# being included in the destination.
exclude=(
    "lib/secret_file.txt"
    "old/embarassing/project/"
)
```
