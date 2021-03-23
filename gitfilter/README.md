# gitfilter

This library eventually will be used in [`mgt`](https://github.com/nikita-skobov/monorepo-git-tools)

This library gives you low level access to the output of [`git-fast-export`](https://www.git-scm.com/docs/git-fast-export)

It is designed to quickly parse the output of `git-fast-export` into structured object which you as the user can then manipulate or filter before passing the output back to some stream.

This stream is meant to eventually be passed into `git-fast-import` which will let you rewrite your git repository history very quickly. This is a very flexible approach because you get access to all commits very quickly, and you can do complicated things such as renaming files/folders, excluding commits, changing times, applying transformations to files, etc, etc.

This library is still a work in progress, but extremely basic functionality works:

```rs
// this example will use the `filter_with_cb` function that
// takes a writable stream, and a callback.
// the writable stream in this case is just standard out
// and we give a callback which will filter out any
// commits where the email address of the committer
// contains "jerry"

use gitfilter::export_parser;
use gitfilter::filter;
use export_parser::StructuredExportObject;
use export_parser::StructuredObjectType;
use std::io::Write;
use std::io::stdout;
use std::io;

fn filter_path_works() {
    let writer = stdout();
    filter_with_cb(writer, |obj| {
        match &obj.object_type {
            StructuredObjectType::Blob(_) => true,
            StructuredObjectType::Commit(commit_obj) => {
                if commit_obj.committer.email.contains("jerry") {
                    false
                } else {
                    true
                }
            }
        }
    }).unwrap();
}
```

This library is heavily inspired from [`git-filter-repo`](https://github.com/newren/git-filter-repo)

This library aims to be a subset of `git-filter-repo`, particularly I don't plan on handling tags, or making a robust CLI for it. Instead, I wrote `gitfilter` because I wanted to use `git-filter-repo` in an advanced way, but `git-filter-repo` is written in python and is a bit slow for my use case, and it requires you to pass in python code as a string for some advanced features. This makes it not very ergonomic for my use case. Additionally, since it is written in python, I am currently listing this as an external dependency in my `mgt` project, but it would be more convenient for users if this filtering functionality was present directly in the `mgt` binary.

