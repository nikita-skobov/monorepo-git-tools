# `mgt verify-rf --help`

```


USAGE:
    mgt verify [FLAGS] [OPTIONS] <repo_file>

FLAGS:
    -h, --help    
    --verbose     show full rename from src -> dest 
    --pretty      format the mapping nicely. implies verbose. wont work well on small terminals 
                  though 
    --stdin       provide a list of files to verify from stdin, one file per line. By default we 
                  get this list of files for you via: git ls-tree -r HEAD --name-only --full-tree 
                  You can achieve the default behavior by doing: git ls-tree -r HEAD --name-only 
                  --full-tree | mgt verify-rf --stdin <PATH/TO/REPOFILE> 

```
