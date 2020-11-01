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
- checking updates of multiple subrepositories
- filtering out specific text/commits before sharing publically (TODO)

## Example

<details>
<summary>(click to expand example)</summary>

The following is a simple example of `mgt` usage. For more information on how to use `mgt`, [see the documentation](./doc/README.md)

We first write a `repo_file` called `meta.rf`:

```toml
[repo]
remote = "https://github.com/nikita-skobov/monorepo-git-tools"

[include_as]
"lib/mgt/src/" = "src/"
"lib/mgt/Cargo.toml" = "Cargo.toml"
```

Next we will run `mgt split-in` to take the remote repository
defined in the above file, fetch it, and rewrite the paths to match our rules according to the `[include_as]` section:

```sh
mgt split-in meta.rf --rebase --num-commits 1
# output:
Pulling from https://github.com/nikita-skobov/monorepo-git-tools
Running filter commands on temporary branch: monorepo-git-tools
Rebasing
Success!
```

We also passed 2 arguments: `--rebase` will automatically rebase the temporary created branch onto our current branch for us, and `--num-commits 1` will only fetch 1 commit from the latest HEAD of the remote repository.

After running the above, we will be in a branch called `monorepo-git-tools` that was created for us, and then rebased such that it can now be fast forwared into master. Let's now merge into master, and then delete the temporary branch:

```sh
git checkout master
git merge --ff-only monorepo-git-tools
```

Now, let's make a commit on the Cargo.toml file that we remapped to `lib/mgt/Cargo.toml`:

```sh
echo "# add a comment to the end of the file" >> lib/mgt/Cargo.toml
git add lib/mgt/Cargo.toml
git commit -m "contributions can be bidirectional!"
```

Now we can check if there are any contributions that we can TAKE from the remote repo according to the mapping defined in our repo file:

```sh
mgt check meta.rf
# output:
---
Checking meta.rf
Current: https://github.com/nikita-skobov/monorepo-git-tools
Upstream: HEAD
You are up to date. Latest commit in current exists in upstream
```

Now let's check if we have any contributions to GIVE to the remote repo:

```sh
mgt check meta.rf --local
# output:
---
Checking meta.rf
Current: HEAD
Upstream: https://github.com/nikita-skobov/monorepo-git-tools
upstream can take 1 commit(s) from current:
ce9a912 contributions can be bidirectional!

To perform this update you can run: 
mgt split-out meta.rf --topbase
```

We can then run the suggested command that `mgt check` outputs to contribute our latest commit back to the remote repository:

```sh
mgt split-out meta.rf --topbase
```

The `--topbase` flag will help calculate which contributions can be applied to the tip of the remote. It also creates a temporary branch that can be used to push to the remote repo. Let's do that with:

```sh
git push https://github.com/nikita-skobov/monorepo-git-tools HEAD:newbranch
```

Which will push our current HEAD to a new remote branch called `newbranch`. Once our changes are up there, we can go back to master and delete our current temporary branch.

```sh
git branch -D monorepo-git-tools
```

That's the end of the example :)

</details>

## How does it work?

`mgt` knows how to rewrite your history based on files that contain information on how to map between local and remote repositories.
These files are called `repo_file`s, and some common things that would
go in a `repo_file` are:

- including files/folders
- renaming files/folders
- excluding files/folders
- specifying repository URL(s) to use as the destination

When you run `mgt`, you typically provide a path to the `repo_file`, eg: `mgt split-out my_repo_file.rf`

## Prerequisites

These commands are just "porcelain" commands built on top of the real "plumbing" capabilities of [git-filter-repo](https://github.com/newren/git-filter-repo).

**You must have `git-filter-repo` available on your path prior to using the tools in this repository.**

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

This project builds and tests every release on [GitHub](https://github.com/nikita-skobov/monorepo-git-tools)

To get the latest binary, go to [the release page](https://github.com/nikita-skobov/monorepo-git-tools/releases) and download
the zip file that is appropriate for your machine.

To install it, simply extract the zip, and copy the executable to a directory that is on your system's path.
