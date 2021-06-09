# `mgt`

> this file was automatically generated on 2021-06-09

Below you'll find a list of links to documentation pages, as well as the
automatically generated output of `mgt --help`


* [split-out](./split-out.md)
* [split-out-as](./split-out-as.md)
* [split-in](./split-in.md)
* [split-in-as](./split-in-as.md)
* [topbase](./topbase.md)
* [check](./check.md)
* [repo_file](./repo_file.md)

## `mgt --help` or
## `mgt -h` or
## `mgt help`

```
mgt 5.0.0 36c4480
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
  sync              Interactively sync one or more repo files between local and remote repositorie(s)
  diff-log          View a log comparing two branches that have potentially unrelated history using a topbase algorithm
  check             check if there are changes ready to be pushed or pulled
  topbase           rebase top branch onto bottom branch but stop the rebase after the first shared commit
  split-in          fetch and rewrite a remote repository's history onto a new branch according to the repo file rules
  split-in-as       fetch and rewrite a remote repository's history onto a new branch and into the --as <subdirectory>
  split-out         create a new branch with this repository's history rewritten according to the repo file rules
  split-out-as      create a new branch with this repository's history rewritten according to the --as <subdirectory>
  verify-repo-file  verify your repo file before running a split operation
  verify-rf         alias for verify-repo-file
```
