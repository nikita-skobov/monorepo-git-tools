# `mgt`

> this file was automatically generated on 2021-01-20

Below you'll find a list of links to documentation pages, as well as the
automatically generated output of `mgt --help`


* [split-out](./split-out.md)
* [split-out-as](./split-out-as.md)
* [split-in](./split-in.md)
* [split-in-as](./split-in-as.md)
* [topbase](./topbase.md)
* [check](./check.md)
* [repo_file](./repo_file.md)
* [verify](./verify-rf.md)

## `mgt --help` or
## `mgt -h` or
## `mgt help`

```
mgt 4.1.1 f57b568
Nikita Skobov
Git tools that enable easy bidirectional sync between multiple repositories

USAGE:
    mgt [SUBCOMMAND] [OPTIONS]

FLAGS:
    --dry-run        Dont run anything. Just print output of what a run would do. 
    --verbose        More detailed output 
    -h, --help       Prints help information 
    -V, --version    Prints version information 

Available commands:
  help
  check         check if there are changes ready to be pushed or pulled
  topbase       rebase top branch onto bottom branch but stop the rebase after the first shared commit
  split-in      fetch and rewrite a remote repository's history onto a new branch according to the repo file rules
  split-in-as   fetch and rewrite a remote repository's history onto a new branch and into the --as <subdirectory>
  split-out     create a new branch with this repository's history rewritten according to the repo file rules
  split-out-as  create a new branch with this repository's history rewritten according to the --as <subdirectory>
```
