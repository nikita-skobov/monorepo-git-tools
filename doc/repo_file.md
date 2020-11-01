# Repo File

I created `mgt` with the intention of defining `repo_files`
that contain information on how to split out/in local repositories back
and forth between remote repositories. A `repo_file` is a toml file
that has variables that describe how your repository should be split. There is
a slight parsing issue with the toml parser which requires that certain sections
(`include` and `exclude`) to be at least 2 new lines apart from the
sections before them. Alternatively, these sections can be placed before
any other section to avoid this issue.

For example:

```toml
[repo]
name = "my repo"

include = ["this"]
```

The above is bad because there is only one newline between `[repo]` and `include`.
The parser will detect `include` as part of the `[repo]` section. To avoid this issue,
you can do either:

```
# good:
[repo]
name = "my repo"


include = ["this"]

# also good:
include = ["this"]
[repo]
name = "my repo"
```

Here is a commented `repo_file` that explains what some of the common variables do.


```toml
[repo]
# used for: git pull $remote_repo when doing split-in
# and for split-out if using --rebase or --topbase
remote = "https://github.com/myname/myrepo"
# instead of pulling remote_repo from HEAD,
# it can pull from a specific branch instead
branch = "feature/X"
# allows you to specify the name of the branch
# that should be output
name = "git-monorepo-tools"


# (needs 2 empty lines here^ to parse correctly!)
# includes the source repository files/directories
# exactly as is, without changing the paths
# NOTE: directories must have trailing slash
include = [
    "doc/some_file.txt"
    "scripts/",
]

# another example of include:
# include can just be a string:
include = "scripts/"

# includes the source files/folders into the destination files/folders
# ie: use this variable if you wish to rename paths
# so below, lib/cool-lib corresponds to a string with a single space.
# this means that when you split out a repo, it will take everything from
# lib/cool-lib/ and put it in the root of the destination repo, ignoring
# all files/folders other than lib/cool-lib/
# NOTE: to put everything in the root, you must specify a string with a
# single empty space:
[include_as]
"lib/cool-lib/" = " "


# Another example of include_as:
# in this example we rename one of the lib files
# and we also move a directory to a different part of the
# destination
[include_as]
"lib/get_arg.sh" = "lib/get_arg.bsc"
"repos/my_blog/" = "lib/my_blog/"


# (again, we need 2 empty lines here)
# excludes the source files/folders from
# being included in the destination.
exclude = [
    "lib/secret_file.txt",
    "old/embarassing/project/"
]
```
