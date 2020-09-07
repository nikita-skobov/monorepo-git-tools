# `mgt topbase --help`

```
mgt-topbase 
rebases top branch onto bottom branch keeping only the first commits until it finds a commit from top where all blobs
exist in the bottom branch.

USAGE:
    mgt topbase [FLAGS] <base> [top]

FLAGS:
    -d, --dry-run    Print out the steps taken, but don't actually run or change anything.
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    show more detailed logs

ARGS:
    <base>    the branch to rebase onto.
    <top>     the branch that will be rebased. defaults to current branch
```
