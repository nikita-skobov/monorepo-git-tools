# `mgt topbase --help`

```
rebase top branch onto bottom branch but stop the rebase after the first shared commit

USAGE:
    mgt topbase [FLAGS] <base> [top]

FLAGS:
    --dry-run    Print out the steps taken, but don't actually run or change anything. 
    --verbose    Prints verbose information 
    --help       

POSITIONAL:
    <base>    the branch to rebase onto.
    [top]     the branch that will be rebased. defaults to current branch
```
