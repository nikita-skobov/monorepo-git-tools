# mgt (monorepo-git-tools)

> Git tools that enable easy bidirectional sync between multiple repositories

## Table of contents

* [What does it do?](#what-does-it-do)
* [Example](#example)
* [How does it work?](#how-does-it-work)
* [Prerequisites](#prerequisites)
* [Docs](#docs)
* [Installation](#installation)

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

## Example

The following is one of the simplest uses of `mgt`. For more interesting use cases, [see the examples here (TODO)](./examples/README.md)

The simplest usage of `mgt` is to take an entire remote repository and `split-in-as` into a subdirectory of your local repository. If you are in a local git repository, you can run:

```sh
mgt split-in-as https://github.com/nikita-skobov/monorepo-git-tools --as lib/mgt/ --rebase
```

This will:
1. create a new, temporary branch (named `monorepo-git-tools`)
2. fetch the remote repository (`https://github.com/nikita-skobov/monorepo-git-tools`) into that branch
3. rewrite the history of this new branch such that the entire contents exist within `lib/mgt/`
4. the `--rebase` flag tells `mgt` to then rebase that new branch onto whatever branch you started from

After those steps, you will be on the `monorepo-git-tools` branch, and you can merge it however you want back into your starting branch.


## How does it work?

`mgt` knows how to rewrite your history based on files that contain information on how to rewrite the history.
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

## Docs

The full command line usage documentation can be found [here](./doc/README.md)

## Installation

### From source

You will need rust installed

```
git clone https://github.com/nikita-skobov/monorepo-git-tools
cd monorepo-git-tools
cargo build --release
chmod +x ./target/release/mgt
cp ./target/release/mgt /usr/bin/mgt
```

### From binary

This project builds and tests every release on [github](https://github.com/nikita-skobov/monorepo-git-tools)

To install from latest binary, go to [the release page](https://github.com/nikita-skobov/monorepo-git-tools/releases) and download
the asset that is appropriate for your machine.

To install it, simply copy it to a directory that is on your system's path.
