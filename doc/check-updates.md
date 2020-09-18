# `mgt check-updates --help`

```
mgt-check-updates 
check if remote has commits not present in local or vice versa

USAGE:
    mgt check-updates [FLAGS] [OPTIONS] <repo-file>

FLAGS:
    -h, --help       Prints help information
    -l, --local      check if the local branch has commits not present in remote
    -r, --remote     check if the remote has commits not present in this local branch. This is the default
    -V, --version    Prints version information

OPTIONS:
        --local-branch <local-branch>      check updates to/from a specific local branch instead of the current HEAD
    -b, --remote-branch <remote-branch>    check updates to/from a specific remote branch instead of what's in the repo
                                           file

ARGS:
    <repo-file>    path to file that contains instructions of how to split a repository
```
