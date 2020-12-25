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
    -r                     after generating a branch with rewritten history, rebase that branch 
                           such that it can be fast forwarded back into the comparison branch. for 
                           split-in that is the branch you started on. For split-out, that is the 
                           remote branch 
    -t                     like rebase, but it finds a fork point by stopping at the first commit 
                           that two branches have in common. This is useful as an 'update' 
                           mechanism. 

OPTIONS:
    --num-commits N                      when pulling from remote, limit to N commits from the 
                                         current tip. This is probably only useful the first time 
                                         you do a split-in 
    -o, --output-branch OUTPUT-BRANCH    name of branch that will be created with new split 
                                         history 
    --rebase BRANCH-NAME                 like the -r flag, but you can specify the name of the 
                                         branch you want to use as the comparison branch instead 
                                         of using the default 
    --topbase BRANCH-NAME                like the -t flag, but you can specify the name of the 
                                         remote branch that will be used instead of what is 
                                         defined in your repo file 
    --as AS-SUBDIR                       path relative to root of the local repository that will 
                                         contain the entire repository being split 

POSITIONAL:
    <git-repo-uri>    a valid git url of the repository to split in
```
