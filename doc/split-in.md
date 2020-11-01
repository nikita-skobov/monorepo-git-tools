# `mgt split-in --help`

```
mgt-split-in 
fetch and rewrite a remote repository's history onto a new branch such that it only contains certain paths according to
a repo-file

USAGE:
    mgt split-in [FLAGS] [OPTIONS] <repo-file>

FLAGS:
    -d, --dry-run    Print out the steps taken, but don't actually run or change anything.
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    show more detailed logs

OPTIONS:
        --input-branch <branch-name>     split in from a local branch in this repository
        --num-commits <n>                when pulling from remote, limit to n commits from the current tip. This is
                                         probably only useful the first time you do a split-in
    -o, --output-branch <branch-name>    name of branch that will be created with new split history
    -r, --rebase <rebase>                after generating a branch with rewritten history, rebase that branch such that
                                         it can be fast forwarded back into the comparison branch. For split-in, the
                                         comparison branch is the branch you started on. For split-out, the comparison
                                         branch is the remote branch. By specifying a value for <rebase>, you can use a
                                         specific remote branch and override what is in your repo file.
    -t, --topbase <topbase>              like rebase, but it finds a fork point to only take the top commits from the
                                         created branch that dont exist in your starting branch. Optionally pass in the
                                         name of a remote branch to override what is in your repo file.

ARGS:
    <repo-file>    path to file that contains instructions of how to split a repository
```
