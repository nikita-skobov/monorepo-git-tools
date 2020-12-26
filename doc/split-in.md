# `mgt split-in --help`

```
fetch and rewrite a remote repository's history onto a new branch according to the repo file rules

USAGE:
    mgt split-in [FLAGS] [OPTIONS] <repo_file>

FLAGS:
    --verbose              Prints verbose information 
    --dry-run              Print out the steps taken, but don't actually run or change anything. 
    --help                 

OPTIONS:
    --input-branch INPUT-BRANCH          split in from a local branch in this repository 
    --num-commits N                      when pulling from remote, limit to N commits from the 
                                         current tip. This is probably only useful the first time 
                                         you do a split-in 
    -o, --output-branch OUTPUT-BRANCH    name of branch that will be created with new split 
                                         history 

POSITIONAL:
    <repo-file>    path to file that contains instructions of how to split a repository
```
