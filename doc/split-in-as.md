# `mgt split-in-as --help`

```
mgt-split-in-as 
fetch the entirety of a remote repository and place it in a subdirectory of this repository

USAGE:
    mgt split-in-as [FLAGS] [OPTIONS] <git-repo-uri> --as <subdirectory>

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
        --as <subdirectory>              path relative to root of the local repository that will contain the entire
                                         repository being split in
    -o, --output-branch <branch-name>    name of branch that will be created with new split history

ARGS:
    <git-repo-uri>    a valid git url of the repository to split in
```
