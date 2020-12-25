# `mgt check --help`

```
check if there are changes ready to be pushed or pulled

USAGE:
    mgt check [FLAGS] [OPTIONS] <repo_file>

FLAGS:
    --all          if the <repo-file> is a directory, by default mgt only looks for files ending 
                   in .rf, but with the --all flag, you are telling mgt to get any file it finds 
                   from the <repo-file> directory 
    --local        check if the local branch has commits not present in remote 
    --recursive    if the <repo-file> is a directory, get all files in this directory recursively 
    --remote       check if the remote has commits not present in this local branch. This is the 
                   default 
    --help         

OPTIONS:
    --local-branch BRANCH-NAME         check updates to/from a specific local branch instead of 
                                       the current HEAD 
    -b, --remote-branch BRANCH-NAME    check updates to/from a specific remote branch instead of 
                                       what's in the repo file 

POSITIONAL:
    <repo-file>    path to file that contains instructions of how to split a repository
```
