# `mgt split-out --help`

```
mgt-split-out 
rewrite this repository history onto a new branch such that it only contains certain paths according to a repo-file

USAGE:
    mgt split-out [FLAGS] [OPTIONS] <repo-file>

FLAGS:
    -d, --dry-run    Print out the steps taken, but don't actually run or change anything.
    -h, --help       Prints help information
    -r, --rebase     after generating a branch with rewritten history, rebase that branch such that it can be fast
                     forwarded back into the comparison branch. For split-in, the comparison branch is the branch you
                     started on. For split-out, the comparison branch is the remote branch
    -t, --topbase    like rebase, but it finds a fork point to only take the top commits from the created branch that
                     dont exist in your starting branch
    -V, --version    Prints version information
    -v, --verbose    show more detailed logs

OPTIONS:
    -o, --output-branch <branch-name>    name of branch that will be created with new split history

ARGS:
    <repo-file>    path to file that contains instructions of how to split a repository
```
