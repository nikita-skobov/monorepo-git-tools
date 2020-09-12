# Examples of what `mgt` can do

This is a series of examples that can show what `mgt` can do, and what kinds of problems it was made to solve. This document
was designed to be read through from start to finish as each step builds on the last. However, if you wish to just skip to the sections
that are interesting to you, you can use the table of contents below. Otherwise, the first section starts [here.](#purpose)

* [split out a single file](#scenario-01-split-out-file)
* [split out a subdirectory](#scenario-02-split-out-folder)
* [multiple includes](#scenario-03-multiple-includes)
* [include exclude](#scenario-04-include-exclude)
* [include as root](#scenario-05-include-as-root)
* [include as rename and move](#scenario-06-include-as-rename-and-move)
* [split in existing code](#scenario-07-split-in-existing-code)
* [let mgt rebase for us](#scenario-08-let-mgt-rebase-for-us)

## Purpose

`git` has a command called `subtree` that can

> Merge subtrees together and split repository into subtrees

This is useful to

- split out a subdirectory of your local repository into an entire new repository 
- split in an entire remote repository into a subdirectory of your local repository

A problem I had with `git subtree` is that you could not specify multiple subdirectories to include,
and you could not exclude certain parts of the repository that you did not want to include.

`mgt` was originally designed to solve these problems, but has also added a few things on top of that
functionality to make contributing code back and forth between local and remote repositories easier.

We will show those additional features, and why they are useful in a bit, but for now, let's
start by showing how to use the basic features of `mgt`.

## Scenario 01 split out file

We will work with a private, local git repository that has the following directory structure:

```
lib/
lib/projectA/
lib/projectA/src/
lib/projectA/src/test.html
lib/projectA/src/index.html
lib/projectA/README.md
lib/projectA/secret.txt
lib/projectB/
lib/projectB/test/
lib/projectB/test/testfile.js
lib/projectB/README.md
lib/projectB/server.js
lib/projectC/script.sh
config/
config/example.conf
```

Assume every file above was committed on its own. This does not need to be the case when you use `mgt`, but for these examples
it will make the history easier to conceptualize.

Also assume for all examples that we are in the root of the repository unless specified otherwise.

We want to take a portion of our repository, and share it publically. We don't want to push the whole
repository because there are parts of it that contain code that we do not want to share.

We will make a `repo_file` (we will name it repo_file.txt and put it
at the root of our repository) that looks like this:

```sh
# repo_file.txt
repo_name="my-projects"

include="lib/projectA/src/test.html"
```

Note that the `repo_name` variable is the name of the branch that will be created for us by `mgt`.

We set the `include` variable to a single string containing the path (from the root of our local repository)
to the test.html file in projectA.

If we run:

```sh
mgt split-out repo_file.txt
```

`mgt` will create a new branch for us (named my-projects), and checkout to that branch.

The newly created branch will contain the following structure:

```
lib/projectA/src/test.html
```

NOTE: we will still have repo_file.txt in the root of our directory
because we did not commit repo_file.txt, and `mgt` does not modify files that aren't committed.

## Scenario 02 split out folder

Now let's say that we actually changed our mind, and we wanted our new split out repository
to be a bit different. Since `mgt` only modifies the history of a temporary branch, we can easily go back
and try again by going back to master

```sh
git checkout master
```

We edit our `repo_file` to look like this:

```sh
repo_name="my-projects"

include="lib/projectA/src/"
```

Instead of only including the single test.html file, we will now include the entire `lib/projectA/src/` directory.
NOTE: You must have a trailing slash for directories.

We will run our `split-out` command again:

```sh
mgt split-out repo_file.txt
```

**However, this will fail because we already have a branch named `my-projects`**. In this case, `mgt` will not do anything because
it does not want to override a branch that already exists. It will simply exit with an error message.

We can either:
1. delete the `my-projects` branch (which is easy and safe to do because in this example we don't want it anyway)
2. specify an alternate output branch name via the `-o` or `--output-branch`

For this example, let's specify an alternate output branch name:

```sh
mgt split-out repo_file.txt -o temp-branch
```

Now, we are in `temp-branch` and our repository structure looks like:

```
lib/projectA/src/test.html
lib/projectA/src/index.html
```

For all future examples we will start from the `master` branch, and with no other branches.

## Scenario 03 multiple includes

Now, let's say we actually want the `README.md` that is in the root of our `projectA/` folder, but we
**do not** want the `secret.txt` file from showing up in our split out repository. There are 2 ways we can accomplish this.
The first is by using an explicit list of files/paths to include in our `repo_file`:

```
repo_name="my-projects"

include=(
  "lib/projectA/src/"
  "lib/projectA/README.md"
)
```

The `include` variable is now a list of strings. It can include any number of paths that you wish to include, but here
we just have two. 

After running `mgt split-out repo_file.txt`, our repository structure now looks like:

```
lib/projectA/src/test.html
lib/projectA/src/index.html
lib/projectA/README.md
```

## Scenario 04 include exclude

To do the same as Scenario 03, we can exclude files/folders via the `exclude` variable. The paths listed in the `exclude` variable
are excluded **AFTER** the `include` variable. This means that you first include, and then you exclude, so the `exclude` variable
can remove portions of included paths that you do not want. Consider:

```
repo_name="my-projects"

include="lib/projectA/"
exclude="lib/projectA/secret.txt"
```

Which, after the `mgt split-out repo_file.txt` command will produce a repository structure like:

```
lib/projectA/src/test.html
lib/projectA/src/index.html
lib/projectA/README.md
```

NOTE: that the `exclude` variable can also be a list of paths like the `include` variable. In this
example though we only needed a single file to be excluded.

## Scenario 05 include as root

So far we have actually been keeping the repository structure the same, and basically we've just been deleting files that
we didn't want. `mgt` can also let you rename files/paths, and restructure your repository. Let's consider an example where
we want our new, split out repository to be in the root (ie: we don't want it to start with `lib/projectA`).
Here is a `repo_file` that will accomplish that:

```
repo_name="my-projects"

include_as=(
  "lib/projectA/" " "
)

exclude="lib/projectA/secret.txt"
```

We introduce a new variable called `include_as`. `include_as` is always a list of strings, and it must be
specifically formatted. It needs to be an even-lengthed list where the paths with an even index are the source
paths, and the paths with an odd index are the destination paths. In this case, we only have one source, and one
destination. Our source is `lib/projectA/`, and our destination is ` ` a string consisting of a single space.
The single space string is a special case of the `include_as` variable. When `mgt` sees an `include_as` variable
that has a single space, it will interpret that as move everything into the root of the repository.

After running `mgt split-out repo_file.txt`, `mgt` will produce a repository structure like:

```
src/test.html
src/index.html
README.md
```

If you have been paying attention, you might notice something weird going on here. Previously, we learned that
the excluded paths get excluded after the included paths. If that's true, then wouldn't the `include_as` variable get ran
first, and that would move `lib/projectA/*` to the root of the repository, and then the `exclude` variable is referencing
`lib/projectA/secret.txt` which is now located in the root: `secret.txt`. How does this work?

This works because the `include_as` variable gets ran **AFTER** the `exclude` variable. The `include_as` variable is also special because
we actually need to `include` it regularly first, and then `include_as` after we `exclude`.

`mgt` will take all of the source paths (in this case just one: `lib/projectA/`), and `include` them regularly first. And then `include_as` at the very end
to rename the path to what it should be.

## Scenario 06 include as rename and move

The above `include_as` was pretty simple, but we can use `include_as` to significantly restructure our repository. Let's say we
want to move the `test.html` file to the root of the repository, and we want to rename the `src/` directory to `lib/`, we can do that with:

```
repo_name="my-projects"

include_as=(
    "lib/projectA/src/test.html" "test.html"
    "lib/projectA/src/" "lib/"
    "lib/projectA/" " "
)

exclude="lib/projectA/secret.txt"
```

NOTE: For simple `include_as` scenarios, the order that we specified the paths did not really matter. In this case it **does matter**. We have to
specify the test.html move first because we want to take it out of the `src/` directory before we rename it. We also want the `src/` to `lib/` rename to happen before
we move the entire `lib/projectA/` into root.

As a good rule of thumb, you should list your `include_as` in order of most specific (furthest depth in the repository), to least specific (closer to the
root of the repository).

Alternatively, we could have writtenn our `repo_file` more verbosely:

```
repo_name="my-projects"

include_as=(
  "lib/projectA/src/test.html" "test.html"
  "lib/projectA/src/index.html "lib/index.html"
  "lib/projectA/README.md" "README.md"
)
```

This will accomplish the same as the previous `repo_file`, but it has the disadvantage that if we later add files to our
`lib/projectA` we will have to rewrite our `repo_file`.


After running `mgt split-out repo_file.txt`, `mgt` will produce a repository structure like:

```
lib/index.html
test.html
README.md
```


## Scenario 07 split in existing code

Now let's say we are finally happy with the state of our repository, and we are ready to make it public.

We will push it to our remote repository: `https://example.com/my-projects`

We then update our `repo_file` to have the `remote_repo` variable:

```
remote_repo="https://example.com/my-projects"

include_as=(
    "lib/projectA/src/test.html" "test.html"
    "lib/projectA/src/" "lib/"
    "lib/projectA/" " "
)

exclude="lib/projectA/secret.txt"
```

If we run `mgt split-out repo_file.txt` again, it will still work, and still make a `my-projects` branch because that is what
it detects as the repository name via the end of the `remote_repo` variable.

Now that our code exists in both our local repository, and in a remote repository `https://example.com/my-projects`, **what happens if we, or
someone else makes changes on the remote repository?**

How can we bring those changes into our local repository? After all, the repositories have entirely different directory structures, and technically
they have different git histories because `mgt` rewrites the history in order to move/rename/include/exclude paths.

This is where `mgt split-in` helps us.

`mgt split-in` originally was designed to simply take code from some remote source, and put it into our local repository once. However,
it's functionality can be extended as a way to enable us to receive contributions from the upstream remote repository. Let's try it now:

```sh
mgt split-in repo_file.txt
```

This will start by doing the same thing that `split-out` does, mainly it will create a new branch named `my-projects`, but instead of making the new
branch directly from our current `HEAD`, it will make an entirely empty branch. Then it will pull the most recent `HEAD` of the `remote_repo` into the
`my-projects` branch. Then, it will rewrite the history of this branch according to the variables we defined in the `repo_file`.

**This is the exciting part: It will do the exact same mapping but in reverse**

This is extremely useful because this enables us to use the same `repo_file` for both `split-out` and `split-in` commands.

After running the above command, we will be on the `my-projects` branch, and our repository structure will look like:

```
lib/projectA/src/test.html
lib/projectA/src/index.html
lib/projectA/README.md
```

This is the contents of the remote repository remapped to match the structure of our original repository.

## Scenario 07 split in existing code and merging

So far, we have only been looking at the repository structure, but now it will be important to look at the history.

Continuing from where we left off in scenario 06, let's do a git log of our current branch that was just `split-in`:

(remember that originally, we made a single commit for every single file. It doesn't matter how you make your commits, but for the sake of this example it is important to point out so that we can track the commits easier as they are mapped between repositories)

```
git log my-projects --oneline
# output:
9c982ae (HEAD -> my-projects) testhtml
17076f7 readme
dd777c1 index.html
```

Let's also look at the git log for the master branch (which is the original repository)

```
git log master --oneline
# output:
9366b5d (master) secret
10124ea testhtml
4cf229f script
3af8e40 server
e8cc01e readme
dbd1b52 testfilejs
a24ad83 readme
664977f index.html
c61b8a2 example.config
```

There are two things to notice here:

1. the branches don't have a shared root commit
2. even the commits that are exactly the same (index.html, testhtml) have different hashes

What happens if we try to merge my-projects into master?

Well as it currently stands, we cannot. However, **what we CAN do is rebase our branch onto master, and THEN merge.**

```
git rebase master
```

This succeeds, and now if we log our current branch again, we will see:

```
git log my-projects --oneline
# output
9366b5d (HEAD -> my-projects, master) secret
10124ea testhtml
4cf229f script
3af8e40 server
e8cc01e readme
dbd1b52 testfilejs
a24ad83 readme
664977f index.html
c61b8a2 example.config
```

Now the my-projects branch is up to date, and compatible with the master branch. If we want, we can go ahead and merge the branch, but as it stands there is nothing new to merge, because they are both at the same commit. Let's simulate a scenario where there might be commits to merge back into our master.

We go back to master, delete the my-projects branch:

```
git checkout master
git branch -D my-projects
```

Now, in a seperate folder, let's clone the remote repository, make some changes, commit, and push them back up:

```
cd some-other-folder
# we add a dot at the end to tell git to clone into our current directory
git clone https://example.com/my-projects .
echo "readme append" >> README.md
git add README.md
git commit -m "readme append"
git push origin master
```

The log of our remote repository now looks like:

```
1387b95 (HEAD -> master) readme append
4831d6e testhtml
a39293a readme
b22ba3b index.html
```


Now let's go back to our original repository, and see if we can pull the recent changes into our original local repository

```
cd -
mgt split-in repo_file.txt
```

We are now in the my-projects branch, and our history looks like:

```
4ea76c9 (HEAD -> my-projects) readme append
9c982ae testhtml
17076f7 readme
dd777c1 index.html
```

Again, it's important to remember that the hashes we see in the my-projects **branch** are different than the hashes we saw
in the my-projects **repository** because even though these commits modified the same files, and have the same commit messages,
**the files are located in different places which affects the commit hash**. But that is not important in most cases because `git rebase`
is smart enough to figure out how to rebase for us. Let's rebase our current my-projects branch onto master and look at the log:

```
git rebase master
git log my-projects --oneline
# output:
781b749 (HEAD -> my-projects) readme append
9366b5d (master) secret
10124ea testhtml
4cf229f script
3af8e40 server
e8cc01e readme
dbd1b52 testfilejs
a24ad83 readme
664977f index.html
c61b8a2 example.config
```

This is exactly what we wanted. **We were able to use `mgt split-in` to restructure our remote repository to match the structure of our local repository** and we **used `git rebase` to make the temporary branch compatible with master so that we can merge the recent changes**. We can merge however we want, ie: a ff-merge, make a merge commit, or squash all remote commits into one.


## Scenario 08 let mgt rebase for us

Instead of merging the recent commit into master, let's demonstrate an alternate way of doing the previous scenario.
`mgt` has a convenience option to rebase for us.

We will go back to our local master branch, delete the my-projects branch, and then `split-in` again:

```
git checkout master
git branch -D my-projects
mgt split-in repo_file.txt --rebase
```

Notice we added an option to the `split-in` command: **`--rebase`**

The `--rebase` option will add a `git rebase <starting-branch>` step after it creates the temporary branch.
So in this example, the temporary branch is my-projects, and the branch that we started from was master. So
`mgt` will rebase my-projects onto master. We can do a git log again:

```
32552fc (HEAD -> my-projects) readme append
9366b5d (master) secret
10124ea testhtml
4cf229f script
3af8e40 server
e8cc01e readme
dbd1b52 testfilejs
a24ad83 readme
664977f index.html
c61b8a2 example.config
```

Here we see that our temporary branch: my-projects is already rebased on top of master, and is ready to be merged.

For this, and the next example, let's use a squash merge (even though there is only one commit to squash here, but in real world
uses you will probably be squashing more than one)

```
git checkout master
git merge my-projects --squash
# this leaves us in a staged state, so
# we are ready to make our squash commit:
git commit -m "squashed readme changes from remote"
```

Now our local repository's master git log looks like:

```
d6a5055 (master) squashed readme changes from remote
9366b5d secret
10124ea testhtml
4cf229f script
3af8e40 server
e8cc01e readme
dbd1b52 testfilejs
a24ad83 readme
664977f index.html
c61b8a2 example.config
```
