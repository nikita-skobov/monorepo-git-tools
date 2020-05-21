# git split in <repo_file> [OPTIONS]

## Options:

```
--dry-run
It doesn't do anything except output the commands that it would take without dry run. You can use this to see what it is doing before it does something.

--output-branch, -o
eg: --output-branch new-branch
By default, split out will make an output branch named:
"$repo_name"-reverse
from your repo_file. If this branch name already exists, you can either specify --output-branch, or you can delete your existing branch of that name.

--merge-branch, -m
eg: --merge-branch some-branch
Allows you to use a local branch as the remote source instead of an actual remote git uri. By providing this option, git split in will simply do:
git merge <branch_name>
instead of:
git pull <remote_repo>
```

## Description

given a `repo_file` that defines how a repository should be split, `git split in` will take the `remote_repo` (or `github.com/$username/$repo_name` if `remote_repo` is not provided), and create a new empty branch based on the remote_repo. Then it will take all of the `include` and `include_as` paths defined in the `repo_file` and rewrite the repository's history on the branch that it just created. This new branch will only include the history for the files specified in the `repo_file`.

You will then be left with a branch that can be considered as its own seperate repository (because it most likely doesn't share any common ancestor commits with your previous branch).


## Example

For example, consider a `repo_file`:

```sh
# my_repo_file.sh
remote_repo="https://github.com/myusername/my-project"
repo_name="my-project"

include_as=(
    "lib/my-project" ""
)
```

Then `git split in my_repo_file.sh` **will create a new EMPTY branch it "my-project-reverse"**, and then it **will pull the `remote_repo` into my-project-reverse**. Next, it **will rewrite the history of `my-project-reverse` such that everything within the repository will be put into a folder called `lib/my-project`**. **It WILL NOT modify any branch other than the one it creates.**

## Use cases

This command is useful for:

- taking public code from a remote location, renaming it to match your internal code structure, and bringing it in to your project while preserving the history.
- pull in changes from one of your monorepo projects
