# git-monorepo-tools

> A collection of custom git commands that I use to maintain my monorepo.

## Table of contents

* [Prerequisites](#prerequisites)
* [What does it do?](#what-does-it-do)
* [Installation](#installation)
* [What is a "repo file"?](#what-is-a-repo-file)
* [Development](#development)

## Prerequisites

These git commands are just "porcelain" commands built on top of the real "plumbing" capabilities of [git-filter-repo](https://github.com/newren/git-filter-repo).

You must have `git-filter-repo` available on your path prior to using the tools in this repository.

## What does it do?

The main tool provided in this project is `git split` which can push out your monorepo into smaller subprojects, as well as bring in changes from your subprojects into your monorepo. Along with `git split` is provided `git topbase` which makes it easier to rebase changes from your subprojects back into your monorepo.

For a more in-depth explanation, the full manuals can be found here:

- [git-split](https://htmlpreview.github.io/?https://github.com/nikita-skobov/git-monorepo-tools/blob/master/dist/git-split.html)
- [git-topbase](https://htmlpreview.github.io/?https://github.com/nikita-skobov/git-monorepo-tools/blob/master/dist/git-topbase.html)

## Installation

This repository will contain periodic stable builds of the source code. The builds will be located in the `dist/` directory. The files in the `dist/` directory are bash shell scripts, and are named according to how git handles subcommands, ie: `git-<subcommand>`. 

The proper way to install these git commands is to put them in the directory of all of the other git commands. You can check this by running `git --exec-path`

Then simply copy the contents of `dist/` to this directory. In my case, it is `/usr/lib/git-core`:

```sh
git clone https://github.com/nikita-skobov/git-monorepo-tools
cd git-monorepo-tools

# replace /usr/lib/git-core with whatever the
# output of 'git --exec-path' is on your system
# !(*.*) will match all files without an extension
sudo cp dist/!(*.*) /usr/lib/git-core
# OR you can just manually copy them:
# sudo cp dist/git-split /usr/lib/git-core
# sudo cp dist/git-topbase /usr/lib/git-core

# to install the man pages:
sudo cp dist/*.gz /usr/share/man/man1
```

## What is a "repo file"?

I created these tools with the intention of defining `repo_file`s that contain information on how to split out/in local repositories back and forth from remote repositories. A `repo_file` is just a shell script that contains some variables. It is sourced by commands in this repository, and the variables that it sources are used to do the splitting out/in.

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

# if remote_repo is not provided,
# use this as the GitHub username when pulling
username="nikita-skobov"

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

## Development

This project uses my library: [bash-source-combine](https://github.com/nikita-skobov/bash-source-combine). Bash source combine lets you write relatively neat and compact bash code in seperate files using an import syntax. These files are then combined into one single output. The files in this repository are mostly `.bsc` files, which stands for bash source combine.

If you want to develop on this project, please do not make any changes
to the `dist/` directory, as these are the built files. Instead, edit any of the `.bsc` files. And then to compile/run them, you will need to install bash-source-combine.

To generate the output script, simply run:

```sh
source_combine git-split.bsc > dist/git-split
```

Or, to compile and run in place:

```sh
run_source_combine git-split.bsc --any-args-you-want
```

To run the tests:

```sh
# make sure you are in the root of the repository
# you need bats installed, see test/README.md
./test/run_tests.sh
```

To generate up to date man pages:

```sh
# make sure you are in the root of the repository
# you need groff, and gzip installed
./doc/run_docs.sh
```
