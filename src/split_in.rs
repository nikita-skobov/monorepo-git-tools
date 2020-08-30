use clap::ArgMatches;

use super::commands::INPUT_BRANCH_ARG;
use super::commands::AS_SUBDIR_ARG;
use super::commands::REPO_URI_ARG;
use super::split::panic_if_array_invalid;
use super::split::Runner;
use super::git_helpers;
use super::exec_helpers;
use super::split::try_get_repo_name_from_remote_repo;
use super::repo_file::RepoFile;

pub trait SplitIn {
    fn validate_repo_file(self) -> Self;
    fn generate_arg_strings(self) -> Self;
    fn make_and_checkout_output_branch(self) -> Self;
    fn populate_empty_branch_with_remote_commits(self) -> Self;
    fn rebase(self) -> Self;
    fn save_current_ref(self) -> Self;
    fn topbase(self) -> Self;
}

impl<'a> SplitIn for Runner<'a> {
    fn validate_repo_file(mut self) -> Self {
        self.input_branch = match self.matches.value_of(INPUT_BRANCH_ARG) {
            None => None,
            Some(branch_name) => {
                match &self.repo {
                    None => panic!("Failed to find repo for some reason"),
                    Some(ref repo) => {
                        if ! git_helpers::branch_exists(branch_name, repo) {
                            panic!("You specified an input branch of {}, but that branch was not found", branch_name);
                        }
                        Some(branch_name.into())
                    },
                }
            },
        };

        let missing_output_branch = self.output_branch.is_none();
        let missing_input_branch = self.input_branch.is_none();
        let missing_repo_name = self.repo_file.repo_name.is_none();
        let missing_remote_repo = self.repo_file.remote_repo.is_none();
        let missing_include_as = self.repo_file.include_as.is_none();
        let missing_include = self.repo_file.include.is_none();

        if missing_remote_repo && missing_input_branch {
            panic!("Must provide either repo_name in your repofile, or specify a --{} argument", INPUT_BRANCH_ARG);
        }

        if missing_include && missing_include_as {
            panic!("Must provide either include or include_as in your repofile");
        }

        if missing_repo_name && !missing_remote_repo && missing_output_branch {
            let output_branch_str = try_get_repo_name_from_remote_repo(
                self.repo_file.remote_repo.clone().unwrap()
            );
            self.repo_file.repo_name = Some(output_branch_str.clone());
            self.output_branch = Some(output_branch_str);
        } else if ! missing_repo_name {
            // make the repo_name the output branch name
            self.output_branch = Some(self.repo_file.repo_name.clone().unwrap());
        } else if ! missing_input_branch {
            // make the output_branch the name of the input_branch -reverse
            let output_branch_str = format!("{}-reverse", self.input_branch.clone().unwrap());
            self.output_branch = Some(output_branch_str);
        }

        panic_if_array_invalid(&self.repo_file.include, true, "include");
        panic_if_array_invalid(&self.repo_file.include_as, false, "include_as");

        self
    }

    // get the current ref that this git repo is pointing to
    // save it for later
    fn save_current_ref(mut self) -> Self {
        self.repo_original_ref = match self.repo {
            Some(ref repo) => get_current_ref(repo),
            None => None,
        };
        self
    }

    fn generate_arg_strings(mut self) -> Self {
        let include_as_arg_str = generate_split_out_arg_include_as(&self.repo_file);
        let exclude_arg_str = generate_split_out_arg_exclude(&self.repo_file);
        let include_arg_str = generate_split_out_arg_include(&self.repo_file);

        if self.verbose {
            println!("{}include_arg_str: {}", self.log_p, include_arg_str);
            println!("{}include_as_arg_str: {}", self.log_p, include_as_arg_str);
            println!("{}exclude_arg_str: {}", self.log_p, exclude_arg_str);
        }

        if include_arg_str != "" {
            self.include_arg_str = Some(include_arg_str);
        }
        if include_as_arg_str != "" {
            self.include_as_arg_str = Some(include_as_arg_str);
        }
        if exclude_arg_str != "" {
            self.exclude_arg_str = Some(exclude_arg_str);
        }

        self
    }

    fn make_and_checkout_output_branch(mut self) -> Self {
        let output_branch_name = self.output_branch.clone().unwrap();

        if self.dry_run {
            println!("git checkout --orphan {}", output_branch_name);
            println!("git rm -rf . > /dev/null");
            return self;
        }

        match self.repo {
            Some(ref r) => {
                let success = git_helpers::make_orphan_branch_and_checkout(
                    output_branch_name.as_str(),
                    r,
                ).is_ok();
                if ! success {
                    panic!("Failed to checkout orphan branch");
                }
                // on a new orphan branch our existing files appear in the stage
                // we need to essentially do "git rm -rf ."
                let success = git_helpers::remove_index_and_files(r).is_ok();
                if ! success {
                    panic!("Failed to remove git indexed files after making orphan");
                }
            },
            _ => panic!("Something went horribly wrong!"),
        };
        if self.verbose {
            println!("{}created and checked out orphan branch {}", self.log_p, output_branch_name);
        }

        self
    }

    fn populate_empty_branch_with_remote_commits(self) -> Self {
        let remote_repo = self.repo_file.remote_repo.clone();
        let remote_branch: Option<&str> = match &self.repo_file.remote_branch {
            Some(branch_name) => Some(branch_name.as_str()),
            None => None,
        };

        match self.repo {
            None => panic!("Failed to find repo?"),
            Some(ref r) => {
                match (self.dry_run, &self.input_branch) {
                    (true, Some(branch_name)) => println!("git merge {}", branch_name),
                    (true, None) => println!("git pull {}", remote_repo.unwrap()),
                    (false, Some(branch_name)) => { git_helpers::merge_branches(&r, &branch_name[..], None); },
                    (false, None) => { git_helpers::pull(&r, &remote_repo.unwrap()[..], remote_branch); },
                };
            },
        };
        self
    }

    fn rebase(self) -> Self {
        let upstream_branch = match self.repo_original_ref {
            Some(ref branch) => branch,
            None => {
                println!("Failed to get repo original ref. Not going to rebase");
                return self;
            },
        };

        if self.verbose {
            println!("rebasing onto {}", upstream_branch);
        }
        if self.dry_run {
            // since we are already on the rebase_from_branch
            // we dont need to specify that in the git command
            // the below command implies: apply rebased changes in
            // the branch we are already on
            println!("git rebase {}", upstream_branch);
            return self
        }

        let args = [
            "git", "rebase", upstream_branch.as_str(),
        ];
        match exec_helpers::execute(&args) {
            Err(e) => println!("Failed to rebase: {}", e),
            Ok(_) => (),
        };
        self
    }

    fn topbase(self) -> Self {
        let repo = match self.repo {
            Some(ref r) => r,
            None => panic!("failed to get repo?"),
        };

        let current_branch = match get_current_ref(repo) {
            Some(s) => s,
            None => {
                println!("Failed to get current branch. not going to rebase");
                return self;
            },
        };

        let upstream_branch = match self.repo_original_ref {
            Some(ref branch) => branch.clone(),
            None => {
                println!("Failed to get repo original ref. Not going to rebase");
                return self;
            },
        };

        let all_commits_of_upstream = match git_helpers::get_all_commits_from_ref(repo, upstream_branch.as_str()) {
            Ok(v) => v,
            Err(e) => panic!("Failed to get all commits! {}", e),
        };
        let all_commits_of_current = match git_helpers::get_all_commits_from_ref(repo, current_branch.as_str()) {
            Ok(v) => v,
            Err(e) => panic!("Failed to get all commits! {}", e),
        };

        let mut num_commits_to_take = 0;
        let mut rebase_data = vec![];
        // for every commit in the current branch (the branch going to be rebased)
        // check if every single tree of every commit exists in the upstream branch.
        // as soon as we a tree of this current branch that exists
        // in upstream, then we break, and run out interactive rebase that we
        // are building
        for c in all_commits_of_current {
            match c.tree() {
                Ok(tree) => {
                    if all_trees_exist(&tree, &all_commits_of_upstream) {
                        break;
                    }
                    num_commits_to_take += 1;
                    let rebase_interactive_entry = format!("pick {} {}", c.id(), c.message().unwrap());
                    rebase_data.push(rebase_interactive_entry);
                },
                _ => (),
            };
        }
        // idk some weird borrow/lifetime issue. i have to drop this here...
        drop(all_commits_of_upstream);

        // need to reverse it because git rebase interactive
        // takes commits in order of oldest to newest, but
        // we parsed them from newest to oldest
        rebase_data.reverse();

        // we just want to use the actual branch names, not the ref name
        let current_branch = current_branch.replace("refs/heads/", "");
        let upstream_branch = upstream_branch.replace("refs/heads/", "");

        if self.dry_run || self.verbose {
            // since we are already on the rebase_from_branch
            // we dont need to specify that in the git command
            // the below command implies: apply rebased changes in
            // the branch we are already on
            println!("rebase_data=\"{}\"", rebase_data.join(""));
            println!("GIT_SEQUENCE_EDITOR=\"echo $rebase_data >\" git rebase -i --onto {} {}~{} {}",
                upstream_branch,
                current_branch,
                num_commits_to_take,
                current_branch,
            );
            if self.dry_run {
                return self;
            }
        }

        // rebase_data="pick <hash> <msg>
        // pick <hash> <msg>
        // pick <hash> <msg>
        // "
        // rebase_command="echo \"$rebase_data\""
        // GIT_SEQUENCE_EDITOR="$rebase_command >" git rebase -i --onto bottom top~3 top
        let upstream_arg = format!("{}~{}", current_branch, num_commits_to_take);
        let args = [
            "git", "rebase", "-i",
            "--onto", upstream_branch.as_str(),
            upstream_arg.as_str(),
            current_branch.as_str(),
        ];
        let rebase_data_str = rebase_data.join("");
        let rebase_data_str = format!("echo \"{}\" >", rebase_data_str);

        match exec_helpers::execute_with_env(
            &args,
            &["GIT_SEQUENCE_EDITOR"],
            &[rebase_data_str.as_str()],
        ) {
            Err(e) => println!("Failed to rebase: {}", e),
            Ok(o) => {
                if o.status != 0 {
                    println!("Failed to rebase: {}", o.stdout);
                }
            },
        };
        self
    }
}

pub fn get_current_ref(repo: &git2::Repository) -> Option<String> {
    let current_branch_res = git_helpers::get_current_branch(repo);
    if let Err(e) = current_branch_res {
        panic!("Failed to get current branch: {}", e);
    }
    Some(current_branch_res.unwrap())
}

// return true if the given tree entry exists
// in any of the commits
pub fn tree_entry_exists_in_commits(
    te: &git2::TreeEntry,
    commits: &Vec<git2::Commit>,
) -> bool {
    for c in commits {
        match c.tree() {
            Ok(ctree) => {
                for cte in ctree.iter() {
                    if te.id() == cte.id() {
                        return true;
                    }
                }
            },
            _ => (),
        }
    }

    false
}

// check if ALL of the given tree entries
// in the tree exist SOMEWHERE in the vec of commits
pub fn all_trees_exist(
    tree: &git2::Tree,
    commits: &Vec<git2::Commit>,
) -> bool {
    for te in tree.iter() {
        if ! tree_entry_exists_in_commits(&te, commits) {
            return false;
        }
    }

    true
}

// iterate over both the include, and include_as
// repofile variables, and generate an overall
// include string that can be passed to
// git-filter-repo
// include gets taken as is, but for include_as, we only care about
// what the destination is because we will run
// the include filter after we rename, so we want the renamed versions
pub fn generate_split_out_arg_include(repofile: &RepoFile) -> String {
    let start_with: String = "--path ".into();
    let include_str = match &repofile.include {
        Some(v) => format!("{}{}", start_with.clone(), v.join(" --path ")),
        None => "".to_string(),
    };

    // include_as is more difficult because the indices matter
    // for splitting out, the even indices are the local
    // paths, so those are the ones we want to include
    let include_as_str = match &repofile.include_as {
        Some(v) => format!("{}{}",
            start_with.clone(),
            v.iter().step_by(2)
                .cloned().collect::<Vec<String>>()
                .join(" --path "),
        ),
        None => "".to_string(),
    };

    // include a space between them if include_str isnt empty
    let seperator: String = if include_str.is_empty() {
        "".into()
    } else {
        " ".into()
    };

    format!("{}{}{}", include_str, seperator, include_as_str)
}


// iterate over the include_as variable, and generate a
// string of args that can be passed to git-filter-repo
pub fn generate_split_out_arg_include_as(repofile: &RepoFile) -> String {
    let include_as = if let Some(include_as) = &repofile.include_as {
        include_as.clone()
    } else {
        return "".into();
    };

    // for split-in src/dest is reversed from spit-out
    // sources are the odd indexed elements, dest are the even
    let sources = include_as.iter().skip(1).step_by(2);
    let destinations = include_as.iter().skip(0).step_by(2);
    assert_eq!(sources.len(), destinations.len());

    let pairs = sources.zip(destinations);
    // pairs is a vec of tuples: (src, dest)
    // when mapping, x.0 is src, x.1 is dest
    format!("--path-rename {}",
        pairs.map(|x| format!("{}:{}", x.0, x.1))
            .collect::<Vec<String>>()
            .join(" --path-rename ")
    )
}

pub fn generate_split_out_arg_exclude(repofile: &RepoFile) -> String {
    let start_with: String = "--invert-paths --path ".into();
    match &repofile.exclude {
        Some(v) => format!("{}{}", start_with.clone(), v.join(" --path ")),
        None => "".to_string(),
    }
}

pub fn run_split_in(matches: &ArgMatches) {
    let runner = Runner::new(matches)
        .get_repo_file()
        .save_current_dir()
        .get_repository_from_current_dir()
        .save_current_ref()
        .verify_dependencies()
        .validate_repo_file()
        .change_to_repo_root()
        .make_and_checkout_output_branch()
        .populate_empty_branch_with_remote_commits()
        .generate_arg_strings()
        .filter_exclude()
        .filter_include_as()
        .filter_include();

    // if we should rebase (or topbase), we need to refresh the repository index
    // since the above filtering commands changed some stuff that
    // our in-memory repository representation does not know about
    // idk if this is the best way to do it, but its simplest
    if runner.should_topbase {
        runner.get_repository_from_current_dir().topbase();
    } else if runner.should_rebase {
        runner.get_repository_from_current_dir().rebase();
    }
}

pub fn run_split_in_as(matches: &ArgMatches) {
    // should be safe to unwrap because its a required argument
    let include_as_src = matches.value_of(AS_SUBDIR_ARG).unwrap();
    let repo_uri = matches.value_of(REPO_URI_ARG).unwrap();
    let mut runner = Runner::new(matches);
    runner.repo_file.include_as = Some(vec![
        include_as_src.into(), " ".into(),
    ]);
    runner.repo_file.remote_repo = Some(repo_uri.into());

    let runner = runner.save_current_dir()
        .get_repository_from_current_dir()
        .save_current_ref()
        .verify_dependencies()
        .validate_repo_file()
        .change_to_repo_root()
        .make_and_checkout_output_branch()
        .populate_empty_branch_with_remote_commits()
        .generate_arg_strings()
        .filter_exclude()
        .filter_include_as()
        .filter_include();

    if runner.should_topbase {
        runner.get_repository_from_current_dir().topbase();
    } else if runner.should_rebase {
        runner.get_repository_from_current_dir().rebase();
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "Must provide either repo_name in your repofile, or specify a")]
    fn should_panic_if_missing_input_branch_and_remote_repo() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let repofile = RepoFile::new();
        runner.repo_file = repofile;
        runner.validate_repo_file();
    }

    #[test]
    fn should_format_include_as_correctly() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec![
            "path/will/be/created/".into(), " ".into(),
        ]);
        runner.repo_file = repofile;
        runner = runner.generate_arg_strings();
        assert_eq!(runner.include_as_arg_str.unwrap(), "--path-rename  :path/will/be/created/");
    }

    // even if user only provides include_as, that is just for renaming
    // we also need to translate that to an include step so that
    // we include only the things the user specifies, otherwise
    // we would just have renamed some folders/files
    #[test]
    fn generate_arg_strings_should_make_an_include_from_include_as() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        // ensure we dont actually run anything
        runner.dry_run = true;
        let mut repofile = RepoFile::new();
        // include_as has dest:src for split in
        // because its the reverse of the split out
        repofile.include_as = Some(vec![
            "locallib/".into(), "lib/".into(),
        ]);
        runner.repo_file = repofile;
        runner = runner.generate_arg_strings();
        let include_arg_str_opt = runner.include_arg_str.clone();
        let include_as_arg_str_opt = runner.include_as_arg_str.clone();

        assert_eq!(include_as_arg_str_opt.unwrap(), "--path-rename lib/:locallib/");
        assert_eq!(include_arg_str_opt.unwrap(), "--path locallib/");
    }

    // // TODO: add this functionality. kinda annoying since it would need
    // // to exist across several methods...
    // //
    // // we cant do this for all include_as since it can be files or folders
    // // but in the case where user is bringing entire repo into a subdir
    // // (ie: include_as=("subdir" " ") we can detect this case
    // // so if the user forgets to place a trailing slash (which
    // // it needs, otherwise filter-repo will not work), then we should
    // // add one for them
    // #[test]
    // fn should_append_trailing_slash_if_missing_for_entire_repo_case() {
    //     let matches = ArgMatches::new();
    //     let mut runner = Runner::new(&matches);
    //     // ensure we dont actually run anything
    //     runner.dry_run = true;
    //     let mut repofile = RepoFile::new();
    //     // include_as has dest:src for split in
    //     // because its the reverse of the split out
    //     repofile.include_as = Some(vec![
    //         "locallib".into(), " ".into(),
    //     ]);
    //     runner.repo_file = repofile;
    //     runner = runner.generate_arg_strings();
    //     let include_arg_str_opt = runner.include_arg_str.clone();
    //     let include_as_arg_str_opt = runner.include_as_arg_str.clone();

    //     assert_eq!(include_as_arg_str_opt.unwrap(), "--path-rename  :locallib/");
    //     assert_eq!(include_arg_str_opt.unwrap(), "--path locallib/");
    // }
}
