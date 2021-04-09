# `mgt split-out-as --help`

```
create a new branch with this repository's history rewritten according to the --as <subdirectory>

USAGE:
    mgt split-out-as [FLAGS] --as <subdirectory> --output-branch <branch-name>

FLAGS:
    --verbose              Prints verbose information 
    --dry-run              Print out the steps taken, but don't actually run or change anything. 
    -h, --help             

OPTIONS:
    -o, --output-branch OUTPUT-BRANCH    name of branch that will be created with new split 
                                         history 
    --as AS-SUBDIR                       path relative to root of the local repository that will 
                                         contain the entire repository being split 

```
