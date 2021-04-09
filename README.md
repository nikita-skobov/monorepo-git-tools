# mgt (monorepo-git-tools)

> Git tools that enable easy bidirectional sync between multiple repositories

`mgt` is a command line tool that makes it easier to maintain a monorepo and contribute code back and forth with other git repositories. It only depends on [`git`](https://git-scm.com/downloads) being installed, and works on Windows, Mac, and Linux.

Most commands in `mgt` depend on a `repo_file` which is a toml file that defines how your repository should be mapped.

```toml
[repo]
remote = "https://github.com/nikita-skobov/gitfilter"


[include_as]
"lib/gitfilter/" = " "
"lib/exechelper/" = "exechelper/"
"lib/die/" = "die/"
"lib/gumdrop/" = "gumdrop/"
```

The above would create the following mapping between your local code, and code that you would push somewhere else (such as Github for example):

```
Local repo               Github repo
================================================
lib/                     die/         
   gitfilter/            exechelper/
      README.md          gumdrop/
      src/               src/
   exechelper/           README.md
   die/
   gumdrop/
   other/
   libs/
doc/
```

Note that there are other folders in the local repo that do not get mapped to the Github repo because they were not defined in the `[include_as]` section. **`mgt` allows you to pick and choose what part of your git history you want to share publicly**

## Jump to

* [Installation](#installation)
* [Command line usage](./doc/README.md)

## Examples of what you can do with `mgt`

* [Easily add remote projects to your private codebase](#easily-add-remote-projects-to-your-private-codebase)
* [Add only parts of remote projects to your private codebase](#add-only-parts-of-remote-projects-to-your-private-codebase)
* [Split out part of your repository to share publicly](#split-out-and-share-publicly)
* [Conveniently push recent changes to a public repo](#conveniently-push-recent-changes-to-a-public-repo)

### Easily add remote projects to your private codebase

If you have a local repository, and you want to bring in an external dependency, but you want the actual source code of the dependency (instead of just consuming it as a package from a package manager), then you can use `mgt` to conveniently add the whole remote repository to a subdirectory of your local repository:

```sh
mgt split-in-as --as lib/animals/cat/ https://github.com/user/supercat
```

The `split-in-as` command will take a remote repository, in this case a Github URL, and it takes a `--as <FOLDER>` argument which maps the entirety of the `user/supercat` repository to a folder that we define. If this folder doesn't exist, it will be created for you. Here are a before and after diagram of what this command would do:


```
# Before running split-in-as:

Local repo                Github supercat
================================================
lib/                     index.js
    private/             README.md 
                         LICENCE
                         
# After running split-in-as:

Local repo                Github supercat
================================================
lib/                     index.js
    animals/             README.md
        cat/             LICENSE
            README.md
            LICENSE
            index.js
    private/                 
```

We can also add the `--gen-repo-file` option to generate a file that contains the mapping that was performed. This repo file can then be used to modify how we want our future mapping to work, or to perform bidirectional sync by specifying a file instead of typing out the `--as lib/animals/cat <github url>` every time. If we ran the following command:

```sh
mgt split-in-as --gen-repo-file --as lib/animals/cat https://github.com/user/supercat
```

Then `mgt` would perform the same operation as earlier, but it would also generate a repo file in our current directory called `supercat.rf` (it uses the name of the repository being split in). This repo file would have the following contents:

```toml
[repo]
name = "supercat"
remote = "https://github.com/user/supercat"

[include_as]
"lib/animals/cat/" = " "
```

If we wanted to sync in the future, we could just specify this file, instead of specifying the extra command line arguments we did before. ie: we could get latest updates with:

```sh
# notice it is not split-in-as:
mgt split-in supercat.rf
```

### Add only parts of remote projects to your private codebase

Many repositories out there have tons of code in them. Often times there are repositories that are really many projects combined into one. We sometimes might want to only take parts of remote projects without getting parts that we don't care about. To do this, we have to explicitly define a repo file that contains exactly the mapping that we want. Let's write one below:

```toml
# bigproject.rf
[repo]
remote = "https://github.com/user/bigproject

[include_as]
"lib/sound/" = "src/sound/"


exclude = ["src/sound/samples/"]
```

We can then use this repo file to `split-in` the remote project into our local repo. We can do that via:

```sh
mgt split-in bigproject.rf --rebase
```

Note that the `--rebase` command will do a simple `git rebase` for us after the `split-in` happens. There should not be any rebase conflicts as long as the `lib/sound/` folder does not already exist with git history. Otherwise, you would have to resolve those conflicts.

Let's look at a before and after diagram of what this command would do:

```
# Before running split-in:

Local repo                Github bigproject
================================================
lib/                     src/
    private/                 projectA/
                             projectB/
                             sound/
                                 code/
                                 samples/
                                     bigfile.mp3
                                 README.md
                         
# After running split-in:

Local repo                Github bigproject
================================================
lib/                     src/
    private/                 projectA/
    sound/                   projectB/
        code/                sound/
        README.md                code/
                                 samples/
                                     bigfile.mp3
                                 README.md
```

We notice that we only take the `src/sound/` folder, and ignore the other projects from the `bigproject` repo. And also, we explicitly exclude `samples/` because it has big audio files that we don't care about.

### Split out and share publicly

Consider a repository that has some private code as well as some code you pulled in from an open source project.

```toml
# apples.rf
[repo]
remote = "https://github.com/user/my-fork-of-apples"

[include_as]
"apples/" = " "


exclude = ["apples/private-notes.txt"]
```

By running: **`mgt split-out apples.rf`**
`mgt` will create a new branch called `my-fork-of-apples` and it will have entirely new git history where the file structure will now look like:

```
Local repo                Github my-fork-of-apples
================================================
private-folder/           README.md
apples/                   index.js
    README.md
    index.js
    private-notes.txt
private-file.txt
```

Note that your private files and folders do not get shared because they were not included in the repo file mapping. Also it is interesting to note that `apples/private-notes.txt` does not get included because we explicitly exclude it in our repo file, but everything else in the `apples/` folder did get included.


### Conveniently push recent changes to a public repo

Continuing from our previous example, we now have a `my-fork-of-apples` branch, and it has some changes in it that we want to share with our public Github fork. We could do the following:

```sh
git pull --rebase https://github.com/user/my-fork-of-apples
```

But that would only work in limited cases where our recently split-out branch is compatible with the remote version. Consider if we have the following git history:

```
Local repo                Github my-fork-of-apples
================================================
aaaaaaaa fixed bug 1
bbbbbbbb improved docs     yyyyyyyy improved docs
cccccccc copied apples     zzzzzzzz initial commit
```

Our local repository has different commit hashes for the "improved docs" but we see that the commit message is the same. Internally, we can analyze the git blobs and trees and find out that really that commit has the same blobs but maybe different trees, which would cause the commit hash to be different. If we know that `bbbb` and `yyyy` are essentially the same commit, we can then say: "let's only apply the most recent aaaa commit on top of yyyy", which we can do by interactively rebasing, and then only picking the commit that we want (ie: `git pull --rebase=interactive https://github.com/user/my-fork-of-apples)

**Alternatively `mgt` provides a convenient way to calculate this fork point, and do this interactive rebase for you**. Consider if instead of running the `mgt split-out apples.rf` command from before, we ran this instead:

```sh
mgt split-out apples.rf --topbase
```

the `--topbase` flag will tell `mgt` to fetch `my-fork-of-apples` into a temporary branch, compute the fork point from the split-out version of apples, and then apply an interactive rebase using the calculated fork point. **it is called `topbase` because it calculates the top point of where the rebase should happen**.

If we ran the `topbase` command shown above, we would now have the following history on the `my-fork-of-apples` local branch:

```
Local repo                Github my-fork-of-apples
================================================
aaaaaaaa fixed bug 1
yyyyyyyy improved docs     yyyyyyyy improved docs
zzzzzzzz initial commit    zzzzzzzz initial commit
```

Now we see that our local repository is compatible with the github repository, and we are ahead by 1 commit, so we can just do a normal push:

```sh
git push https://github.com/user/my-fork-of-apples HEAD:some-branch
# and then you can do a pull request in Github to merge some-branch
# or alternatively you can just push directly to the main branch
```

## Installation

### From source

You will need rust installed

```
git clone https://github.com/nikita-skobov/monorepo-git-tools
cd monorepo-git-tools
cargo build --release
chmod +x ./target/release/mgt
cp ./target/release/mgt /usr/bin/mgt
```

### From binary

This project builds and tests every release on [GitHub](https://github.com/nikita-skobov/monorepo-git-tools)

To get the latest binary, go to [the release page](https://github.com/nikita-skobov/monorepo-git-tools/releases) and download
the zip file that is appropriate for your machine.

To install it, simply extract the zip, and copy the executable to a directory that is on your system's path.

## Docs

The full command line usage documentation can be found [here](./doc/README.md)
