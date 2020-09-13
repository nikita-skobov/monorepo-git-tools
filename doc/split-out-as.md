# `mgt split-out-as --help`

```
mgt-split-out-as 
make a new repository (via a branch) that only contains commits that are part of a subdirectory

USAGE:
    mgt split-out-as [FLAGS] --as <subdirectory> --output-branch <branch-name>

FLAGS:
    -d, --dry-run    Print out the steps taken, but don't actually run or change anything.
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    show more detailed logs

OPTIONS:
        --as <subdirectory>              path relative to root of the local repository that will contain the entire
                                         repository being split
    -o, --output-branch <branch-name>    name of branch that will be created with new split history
```
