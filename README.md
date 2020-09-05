# mgt (monorepo-git-tools)

> Git tools that enable easy bidirectional sync between multiple repositories

## Table of contents

* [What does it do?](#what-does-it-do)
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
