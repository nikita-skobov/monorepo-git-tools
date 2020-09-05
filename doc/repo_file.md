# Repo File

I created `mgt` with the intention of defining `repo_files`
that contain information on how to split out/in local repositories back
and forth between remote repositories. A `repo_file` is just a text file
that has variables that describe how your repository should be split. The
syntax is bash-like and only supports variables, strings, lists of strings,
and comments.

Here is a commented `repo_file` that explains what some of the common variables do.

For a full documentation on every option, see [here](#repo-file-variables)


```sh
# used for: git pull $remote_repo when doing split-in
# and for split-out if using --rebase or --topbase
remote_repo="https://github.com/myname/myrepo"

# instead of pulling remote_repo from HEAD,
# it can pull from a specific branch instead
remote_branch="feature/X"

# allows you to specify the name of the branch
# that should be output
repo_name="git-monorepo-tools"

# includes the source repository files/directories
# exactly as is, without changing the paths
# NOTE: directories must have trailing slash
include=(
    "doc/some_file.txt"
    "scripts/"
)

# another example of include:
# include can just be a string:
include="scripts/"

# includes the source files/folders into the destination files/folders
# ie: use this variable if you wish to rename paths
# so below, lib/cool-lib corresponds to a string with a single space.
# this means that when you split out a repo, it will take everything from
# lib/cool-lib/ and put it in the root of the destination repo, ignoring
# all files/folders other than lib/cool-lib/
# NOTE: to put everything in the root, you must specify a string with a
# single empty space:
include_as=(
    "lib/cool-lib/" " "
)

# Another example of include_as:
# in this example we rename one of the lib files
# and we also move a directory to a different part of the
# destination
include_as=(
    "lib/get_arg.sh" "lib/get_arg.bsc"
    "repos/my_blog/" "lib/my_blog/"
)

# excludes the source files/folders from
# being included in the destination.
exclude=(
    "lib/secret_file.txt"
    "old/embarassing/project/"
)
```


# Repo File Variables

The following is a list of all valid variables you can use in your repo file.

 ## repo_name
 The name of the remote repository <br/>
 This will be the branch name when mgt creates a temporary branch. <br/>
 Required only if `remote_repo` is not specified.
 ## remote_repo
 A valid git repo uri. Can be a local file location, remote url, ssh url, etc. <br/>
 For `split-in` the git history of `remote_repo` is rewritten to match this local repo's history. <br/>
 For `split-out` the git history of this local repository is rewritten to match the `remote_repo`. <br/>
 Required for `split-in`, only required for `split-out` if using `--topbase` or `--rebase`.
 ## remote_branch
 A name of a branch available on the `remote_repo`. By default `split-in` (and `split-out` if
 using `--topbase` or `--rebase`) use the HEAD of the `remote_repo`, but you can specify a specific
 branch to use instead.
 Optional.
 ## include_as
 A list of paths where even-indexed paths are the sources, and odd-indexed paths are the destinations. <br/>
 The source is a path to a file/folder in this local repository, and the destination is
 a path to a file/folder in the remote repository. <br/>
 This is so that you can use the same `repo_file` for both splitting in and out.

 Examples:
 ```
 include_as=("my_markdown_notes/002-notes-on-this-thing.md" "README.md")
 ```
 When running `split-out` this will rewrite the users repository
 and only keep the file: `my_markdown_notes/002-notes-on-this-thing.md`, however
 when it rewrites the history, it will also rename the file to be `README.md`.
 When running `split-in` this will take the `README.md` file from the `remote_repo`
 and rename it to be `my_markdown_notes/002-notes-on-this-thing.md`
 ```
 include_as=(
     "lib/file1.txt" "file1.txt"
     "lib/project/" " "
 )
 ```
 For `split-out` this will rename the local repository's `lib/file1.txt` to just `file1.txt`
 and it will take the entire folder `lib/project/` and make that the root of the split out repository.
 NOTE that when specifying directories, you MUST include a trailing slash. And if you wish to make a subdirectory
 the root of the split repository, the correct syntax is a single empty space: `" "`.
 ## include
 A list of paths to include. Unlike `include_as`, this does not allow for renaming.
 There is no source/destination here, it is just a list of paths to keep exactly as they are.

 Examples:
 ```
 include=(
    "README.md"
    "LICENSE"
 )
 ```
 This will only take the `README.md` and `LICENSE` files at the root level, and ignore everything else.
 ```
 include="lib/"
 include=("lib/")
 ```
 Both of the above are valid. `include` can be a single string if you only have one path to include.
 ## exclude
 A list of paths to exclude. This is useful if you want a folder, but don't want some of the
 subfolders.

 Examples:
 ```
 include="lib/"
 exclude=("lib/private/" "lib/README.md")
 ```
 For `split-in` this will take the entirety of the `lib/` folder, but will not take `lib/README.md` and
 will not take the entire subfolder `lib/private/`. Note that `exclude` does not make sense for both `split-out`
 and `split-in`. In the above example, if you use this same `repo_file` again to `split-out` your changes,
 you do not have a `lib/private` or a `lib/README.md`, so this `exclude` statement will not do anything.
 This means you can specify both local paths to exclude and remote paths to exclude:
 ```
 exclude=(
    "localfile.txt"
    "remotefile.txt"
 )
 ```
 If your local repository has a `localfile.txt` then `split-out` will not include it, and `split-out` will do
 nothing about the `remotefile.txt` (because there isn't one).<br/>
 If the remote repository has a `remotefile.txt` then that file will be excluded when running `split-in`. <br/>
 NOTE: in the future there might be an `exclude_local` and `exclude_remote` to avoid these ambiguities.
