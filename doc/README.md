# `mgt`

> this file was automatically generated on 2020-09-01

Below you'll find a list of links to documentation pages, as well as the
automatically generated output of `mgt --help`


* [split-out](./split-out.md)
* [split-in](./split-in.md)
* [split-in-as](./split-in-as.md)
* [repo_file](./repo_file.md)

## `mgt --help` or
## `mgt -h` or
## `mgt help`

```
mgt 2.0.0
Nikita Skobov
Git tools that enable easy bidirectional sync between multiple repositories

USAGE:
    mgt [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help           Prints this message or the help of the given subcommand(s)
    split-in       fetch and rewrite a remote repository's history onto a new branch such that it only contains
                   certain paths according to a repo-file
    split-in-as    fetch the entirety of a remote repository and place it in a subdirectory of this repository
    split-out      rewrite this repository history onto a new branch such that it only contains certain paths
                   according to a repo-file
```
