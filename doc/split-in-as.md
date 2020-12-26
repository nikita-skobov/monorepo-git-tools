# `mgt split-in-as --help`

```
fetch and rewrite a remote repository's history onto a new branch and into the --as <subdirectory>

USAGE:
    mgt split-in-as [FLAGS] [OPTIONS] <git-repo-uri> --as <subdirectory>

FLAGS:
    -g, --gen-repo-file    generate a repo file from the provided remote repo and the --as 
                           argument gets mapped to [include_as] 
    --verbose              Prints verbose information 
    --dry-run              Print out the steps taken, but don't actually run or change anything. 
    --help                 

OPTIONS:
    --num-commits N                      when pulling from remote, limit to N commits from the 
                                         current tip. This is probably only useful the first time 
                                         you do a split-in 
    -o, --output-branch OUTPUT-BRANCH    name of branch that will be created with new split 
                                         history 
    --as AS-SUBDIR                       path relative to root of the local repository that will 
                                         contain the entire repository being split 

POSITIONAL:
    <git-repo-uri>    a valid git url of the repository to split in
```
