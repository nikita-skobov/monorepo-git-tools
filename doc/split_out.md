# git split out <repo_file> [OPTIONS]

## Options:

```
--dry-run
It doesn't do anything except output the commands that it would take without dry run. You can use this to see what it is doing before it does something.

--output-branch, -o
By default, split out will make an output branch named:
$repo_name
from your repo_file. If this branch name already exists, you can either specify --output-branch, or you can delete your existing branch of that name.
```

## Description

given a `repo_file` that defines how a repository should be split, `git split out` will take all paths defined in the `repo_file` and rewrite the repository's history into a new branch. This new branch will only include the history for the files specified in the `repo_file`.

You will then be left with a branch that can be considered as its own seperate repository (because it most likely doesn't share any common ancestor commits with your previous branch).


## Example

For example, consider a `repo_file`:

```sh
# my_repo_file.sh
repo_name="my-project"

include=(
    "lib/project_one"
    "lib/project_two"
)

exclude="lib/project_two/secret_file.txt"
```

Then `git split out my_repo_file.sh` **will create a new branch from your current branch and name it "my-project"**, and then it **will remove any commits on this new branch that apply to anything outside of the paths specified**, ie: it will remove anything that DONT apply to `lib/project_one` and `lib/project_two`, and then it will remove any commits that DO apply to `lib/project_two/secret_file.txt`. **It WILL NOT modify any branch other than the one it creates.**

## Use cases

This command is useful for:

- taking private code, and splitting out only the parts you want to be public, and then publishing it
- temporarily renaming your project's paths before contributing your changes to a public project
- push out one of your monorepo projects
