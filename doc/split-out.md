# `mgt split-out --help`

```
create a new branch with this repository's history rewritten according to the repo file rules

USAGE:
    mgt split-out [FLAGS] [OPTIONS] <repo_file>

FLAGS:
    --verbose              Prints verbose information 
    --dry-run              Print out the steps taken, but don't actually run or change anything. 
    --help                 

OPTIONS:
    -o, --output-branch OUTPUT-BRANCH    name of branch that will be created with new split 
                                         history 

POSITIONAL:
    <repo-file>    path to file that contains instructions of how to split a repository
```
