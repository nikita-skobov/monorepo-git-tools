# `mgt split-out-as --help`

```
create a new branch with this repository's history rewritten according to the --as <subdirectory>

USAGE:
    mgt split-out-as [FLAGS] --as <subdirectory> --output-branch <branch-name>

FLAGS:
    --verbose              Prints verbose information 
    --dry-run              Print out the steps taken, but don't actually run or change anything. 
    --help                 
    -r                     after generating a branch with rewritten history, rebase that branch 
                           such that it can be fast forwarded back into the comparison branch. for 
                           split-in that is the branch you started on. For split-out, that is the 
                           remote branch 
    -t                     like rebase, but it finds a fork point by stopping at the first commit 
                           that two branches have in common. This is useful as an 'update' 
                           mechanism. 

OPTIONS:
    -o, --output-branch OUTPUT-BRANCH    name of branch that will be created with new split 
                                         history 
    --as AS-SUBDIR                       path relative to root of the local repository that will 
                                         contain the entire repository being split 

```
