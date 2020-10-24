use clap::ArgMatches;

use super::split::panic_if_array_invalid;
use super::split::Runner;
use super::split::try_get_repo_name_from_remote_repo;
use super::split::has_both_topbase_and_rebase;
use super::repo_file::RepoFile;
use super::git_helpers;
use super::git_helpers3;
use super::commands::AS_SUBDIR_ARG;
use super::commands::OUTPUT_BRANCH_ARG;
use super::die;

pub trait SplitOut {
    fn validate_repo_file(self) -> Self;
    fn generate_arg_strings(self) -> Self;
    fn make_and_checkout_output_branch(self) -> Self;
    fn checkout_output_branch(self) -> Self;
    fn delete_branch(self, branch_name: &str) -> Self;
}

impl<'a> SplitOut for Runner<'a> {
    fn validate_repo_file(mut self) -> Self {
        let missing_output_branch = self.output_branch.is_none();
        let missing_repo_name = self.repo_file.repo_name.is_none();
        let missing_remote_repo = self.repo_file.remote_repo.is_none();
        let missing_include_as = self.repo_file.include_as.is_none();
        let missing_include = self.repo_file.include.is_none();
    
        if missing_remote_repo && missing_repo_name && missing_output_branch {
            die!("Must provide either repo_name or remote_repo in your repofile");
        }
    
        if missing_include && missing_include_as {
            die!("Must provide either include or include_as in your repofile");
        }
    
        if missing_output_branch && missing_repo_name && !missing_remote_repo {
            let output_branch_str = try_get_repo_name_from_remote_repo(
                self.repo_file.remote_repo.clone().unwrap()
            );
            self.repo_file.repo_name = Some(output_branch_str.clone());
            self.output_branch = Some(output_branch_str);
        } else if missing_output_branch && ! missing_repo_name {
            // make the repo_name the output branch name
            self.output_branch = Some(self.repo_file.repo_name.clone().unwrap());
        }
    
        panic_if_array_invalid(&self.repo_file.include, true, "include");
        panic_if_array_invalid(&self.repo_file.include_as, false, "include_as");
        
        self
    }

    fn generate_arg_strings(mut self) -> Self {
        let include_arg_str = generate_split_out_arg_include(&self.repo_file);
        let include_as_arg_str = generate_split_out_arg_include_as(&self.repo_file);
        let exclude_arg_str = generate_split_out_arg_exclude(&self.repo_file);
        if self.verbose {
            println!("{}include_arg_str: {}", self.log_p, include_arg_str.join(" "));
            println!("{}include_as_arg_str: {}", self.log_p, include_as_arg_str.join(" "));
            println!("{}exclude_arg_str: {}", self.log_p, exclude_arg_str.join(" "));
        }

        if include_arg_str.len() != 0 {
            self.include_arg_str = Some(include_arg_str);
        }
        if include_as_arg_str.len() != 0 {
            self.include_as_arg_str = Some(include_as_arg_str);
        }
        if exclude_arg_str.len() != 0 {
            self.exclude_arg_str = Some(exclude_arg_str);
        }
        self
    }

    fn checkout_output_branch(self) -> Self {
        let output_branch_name = self.output_branch.clone().unwrap();
        if self.dry_run {
            println!("git checkout {}", output_branch_name);
            return self;
        }
        let output_branch_ref = format!("refs/heads/{}", output_branch_name);

        match self.repo {
            Some(ref r) => {
                match git_helpers::checkout_to_branch_and_clear_index(
                    output_branch_ref.as_str(),
                    r,
                ) {
                    Ok(_) => (),
                    Err(e) => {
                        die!("Failed to checkout branch {}", e);
                    }
                };
            },
            _ => die!("Something went horribly wrong!"),
        };
        if self.verbose {
            println!("{} checked out branch {}", self.log_p, output_branch_name);
        }

        self
    }

    fn make_and_checkout_output_branch(self) -> Self {
        let output_branch_name = self.output_branch.clone().unwrap();
        if self.dry_run {
            println!("git checkout -b {}", output_branch_name);
            return self;
        }

        match self.repo {
            Some(ref r) => {
                let success = git_helpers3::make_new_branch_from_head_and_checkout(
                    output_branch_name.as_str(),
                ).is_ok();
                if ! success {
                    die!("Failed to checkout new branch");
                }
            },
            _ => die!("Something went horribly wrong!"),
        };
        if self.verbose {
            println!("{}created and checked out new branch {}", self.log_p, output_branch_name);
        }

        self
    }

    fn delete_branch(self, branch_name: &str) -> Self {
        match self.repo {
            Some(ref r) => {
                match git_helpers::delete_branch(branch_name, r) {
                    Err(e) => println!("Failed to delete branch: {}. {}", branch_name, e),
                    Ok(_) => (),
                }
            },
            None => println!("Failed to delete branch: {}", branch_name),
        }
        self
    }
}


// iterate over both the include, and include_as
// repofile variables, and generate an overall
// include string that can be passed to
// git-filter-repo
pub fn generate_split_out_arg_include(repofile: &RepoFile) -> Vec<String> {
    let include = if let Some(include) = &repofile.include {
        include.clone()
    } else {
        vec![]
    };
    let include_as = if let Some(include_as) = &repofile.include_as {
        include_as.clone()
    } else {
        vec![]
    };

    let mut out_vec = vec![];
    for path in include {
        out_vec.push("--path".to_string());
        if path == " " {
            out_vec.push("".into());
        } else {
            out_vec.push(path);
        }
    }
    // include_as is more difficult because the indices matter
    // for splitting out, the even indices are the local
    // paths, so those are the ones we want to include
    for path in include_as.iter().step_by(2) {
        out_vec.push("--path".to_string());
        if path == " " {
            out_vec.push("".into())
        } else {
            out_vec.push(path.into());
        }
    }

    out_vec
}

// iterate over the include_as variable, and generate a
// string of args that can be passed to git-filter-repo
pub fn generate_split_out_arg_include_as(repofile: &RepoFile) -> Vec<String> {
    let include_as = if let Some(include_as) = &repofile.include_as {
        include_as.clone()
    } else {
        return vec![];
    };

    // sources are the even indexed elements, dest are the odd
    let sources = include_as.iter().skip(0).step_by(2);
    let destinations = include_as.iter().skip(1).step_by(2);
    assert_eq!(sources.len(), destinations.len());

    let pairs = sources.zip(destinations);
    // pairs is a vec of tuples: (src, dest)
    // when mapping, x.0 is src, x.1 is dest
    let mut out_vec = vec![];
    for (src, dest) in pairs {
        out_vec.push("--path-rename".to_string());

        // if user provided a single space to indicate
        // move to root, then git-filter-repo wants that
        // as an emptry string
        let use_src = if src == " " { "" } else { src };
        let use_dest = if dest == " " { "" } else { dest };

        out_vec.push(format!("{}:{}", use_src, use_dest));
    }
    out_vec
}

pub fn generate_split_out_arg_exclude(repofile: &RepoFile) -> Vec<String> {
    let mut out_vec = vec![];
    match &repofile.exclude {
        None => return out_vec,
        Some(v) => {
            out_vec.push("--invert-paths".to_string());
            for path in v {
                out_vec.push("--path".to_string());
                if path == " " {
                    out_vec.push("".into());
                } else {
                    out_vec.push(path.clone());
                }
            }
            out_vec
        },
    }
}

pub fn run_split_out(matches: &ArgMatches) {
    if has_both_topbase_and_rebase(matches) {
        die!("Cannot use both --topbase and --rebase");
    }

    let runner = Runner::new(matches)
        .get_repo_file()
        .verify_dependencies()
        .validate_repo_file()
        .save_current_dir()
        .get_repository_from_current_dir()
        .change_to_repo_root()
        .safe_to_proceed()
        .add_label_before_topbase(true)
        .generate_arg_strings()
        .make_and_checkout_output_branch();

    let log_p = runner.log_p.clone();
    let temp_branch = runner.output_branch.clone().unwrap_or("\"\"".into());
    println!("{}Running filter commands on temporary branch: {}", log_p, temp_branch);
    let mut runner = runner
        .filter_include()
        .filter_exclude()
        .filter_include_as();

    // for split out, rebase is a bit different because
    // we actually need to fetch the remote repo|branch that
    // the user specified in the repo file, and then checkout to that branch
    // then save its ref, then checkout back to the newly created branch,
    // then run rebase, then delete the fetched branch since it is not
    // useful to us anymore after the rebase
    let either_rebase_or_topbase = runner.should_rebase || runner.should_topbase;
    if either_rebase_or_topbase {
        // TODO: what if user has a branch with this name...
        let tmp_remote_branch = "mgt-remote-branch-tmp";
        runner = runner.get_repository_from_current_dir()
            .make_and_checkout_orphan_branch(tmp_remote_branch)
            .populate_empty_branch_with_remote_commits()
            .save_current_ref()
            .checkout_output_branch();

        if runner.should_rebase {
            println!("{}Rebasing", log_p);
            runner = runner.rebase().delete_branch(tmp_remote_branch);
        } else if runner.should_topbase {
            println!("{}Topbasing", log_p);
            use super::topbase::Topbase;
            runner = runner.topbase().delete_branch(tmp_remote_branch);
        }

        match runner.status {
            0 => println!("{}Success!", log_p),
            c => {
                std::process::exit(c);
            },
        };
    }
}

pub fn run_split_out_as(matches: &ArgMatches) {
    if has_both_topbase_and_rebase(matches) {
        die!("Cannot use both --topbase and --rebase");
    }

    // should be safe to unwrap because its a required argument
    let include_as_src = matches.value_of(AS_SUBDIR_ARG).unwrap();
    let output_branch = matches.value_of(OUTPUT_BRANCH_ARG[0]).unwrap();
    let mut runner = Runner::new(matches);
    runner.repo_file.include_as = Some(vec![
        include_as_src.into(), " ".into(),
    ]);
    runner.repo_file.repo_name = Some(output_branch.into());

    let mut runner = runner.save_current_dir()
        .get_repository_from_current_dir()
        .verify_dependencies()
        .validate_repo_file()
        .change_to_repo_root()
        .safe_to_proceed()
        .generate_arg_strings()
        .make_and_checkout_output_branch();

    let log_p = runner.log_p.clone();
    let temp_branch = runner.output_branch.clone().unwrap_or("\"\"".into());
    println!("{}Running filter commands on temporary branch: {}", log_p, temp_branch);
    runner = runner.filter_include()
        .filter_exclude()
        .filter_include_as();

    match runner.status {
        0 => println!("{}Success!", log_p),
        c => {
            std::process::exit(c);
        },
    };
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "Must provide either repo")]
    fn should_panic_if_no_repo_name_or_remote_repo() {
        let argmatches = ArgMatches::new();
        let runner = Runner::new(&argmatches);
        runner.validate_repo_file();
    }

    #[test]
    fn should_generate_include_args_properly() {
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec![
            "abc".into(), "abc-remote".into(),
            "xyz".into(), "xyz-remote".into(),
        ]);
        repofile.include = Some(vec!["123".into()]);
        let filter_args = generate_split_out_arg_include(&repofile);
        assert_eq!(filter_args.join(" "), "--path 123 --path abc --path xyz");
    }

    #[test]
    fn should_generate_exclude_args_properly_for_one_exclude() {
        let mut repofile = RepoFile::new();
        repofile.exclude = Some(vec![
            "one".into(),
        ]);
        let filter_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_args.join(" "), "--invert-paths --path one");
    }

    #[test]
    fn should_generate_exclude_args_properly_for_multiple_exclude() {
        let mut repofile = RepoFile::new();
        repofile.exclude = Some(vec![
            "one".into(), "two".into(), "three".into(),
        ]);
        let filter_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_args.join(" "), "--invert-paths --path one --path two --path three");
    }

    // not sure how to test this. I want to test if
    // println was called with certain strings (because dry-run was set true
    // that means it will println instead of running the command)
    // but i dont think you can do that in rust
    // #[test]
    // fn should_not_run_filter_exclude_if_no_exclude_provided() {
    //     let matches = ArgMatches::new();
    //     let mut runner = Runner::new(&matches);
    //     // ensure we dont actually run anything
    //     runner.dry_run = true;
    //     let repofile = RepoFile::new();
    //     runner.repo_file = repofile;
    //     runner = runner.generate_arg_strings();
    //     let exclude_arg_str = runner.exclude_arg_str.clone().unwrap();
    //     assert_eq!(exclude_arg_str, "".to_string());
    // }

    #[test]
    fn should_generate_empty_vecs_for_generating_args_of_none() {
        let mut repofile = RepoFile::new();
        repofile.exclude = None;
        repofile.include = None;
        repofile.include_as = None;

        let filter_exclude_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_exclude_args.len(), 0);

        let filter_include_args = generate_split_out_arg_include(&repofile);
        assert_eq!(filter_include_args.len(), 0);

        let filter_include_as_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_include_as_args.len(), 0);
    }

    // similarly to the above test, if the actual generate_split_out_arg_include_as
    // returns "", then we want the generate_arg_strings method to
    // set the fields to None
    #[test]
    fn should_set_arg_strs_to_none_if_empty_string() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        // ensure we dont actually run anything
        runner.dry_run = true;
        let mut repofile = RepoFile::new();
        repofile.exclude = None;
        repofile.include = None;
        repofile.include_as = None;
        runner.repo_file = repofile;
        runner = runner.generate_arg_strings();
        let exclude_arg_str_opt = runner.exclude_arg_str.clone();
        let include_arg_str_opt = runner.include_arg_str.clone();
        let include_as_arg_str_opt = runner.include_as_arg_str.clone();

        assert_eq!(exclude_arg_str_opt, None);
        assert_eq!(include_arg_str_opt, None);
        assert_eq!(include_as_arg_str_opt, None);
    }

    #[test]
    fn should_generate_split_out_arg_include_as_properly() {
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec![
            "abc-src".into(), "abc-dest".into(),
            "xyz-src".into(), "xyz-dest".into(),
        ]);
        let filter_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_args.join(" "), "--path-rename abc-src:abc-dest --path-rename xyz-src:xyz-dest");
    }

    // if include_as is None, it shouldnt fail, but rather
    // just return an empty string
    #[test]
    fn gen_split_out_arg_include_as_should_not_fail_if_no_include_as() {
        let repofile = RepoFile::new();
        let filter_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_args.len(), 0);
    }

    #[test]
    #[should_panic(expected = "Must provide either include")]
    fn should_panic_if_no_include_or_include_as() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        repofile.repo_name = Some("reponame".into());
        runner.repo_file = repofile;
        runner.validate_repo_file();
    }

    #[test]
    #[should_panic(expected = "Failed to parse repo_name from remote_repo")]
    fn should_panic_if_failed_to_parse_repo_name_from_remote_repo() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        repofile.include = Some(vec!["sdsa".into()]);
        repofile.remote_repo = Some("badurl".into());
        runner.repo_file = repofile;
        runner.validate_repo_file();
    }

    #[test]
    #[ignore = "doesnt work on windows yet"]
    fn should_parse_repo_name_from_remote_repo_if_valid_path() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec!["dsadsa".into(), "dsadsa".into()]);
        repofile.remote_repo = Some("./path/to/reponame".into());
        runner.repo_file = repofile;
        runner = runner.validate_repo_file();
        assert_eq!(runner.repo_file.repo_name.unwrap(), "reponame".to_string());
    }

    #[test]
    #[ignore = "doesnt work on windows yet"]
    fn should_parse_repo_name_from_remote_repo_if_valid_url() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        repofile.include_as = Some(vec!["dsadsa".into(), "dsadsa".into()]);
        repofile.remote_repo = Some("https://website.com/path/to/reponame.git".into());
        runner.repo_file = repofile;
        runner = runner.validate_repo_file();
        assert_eq!(runner.repo_file.repo_name.unwrap(), "reponame".to_string());
    }

    #[test]
    #[should_panic(expected = "Must be either a single string, or")]
    fn should_panic_if_include_is_odd() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        // vec of 3 strings:
        repofile.include = Some(vec!["dsadsa".into(), "dsadsa".into(), "dsadsa".into()]);
        repofile.repo_name = Some("dsadsa".into());
        runner.repo_file = repofile;
        runner.validate_repo_file();
    }

    #[test]
    #[should_panic(expected = "Must be either a single string, or")]
    fn should_panic_if_include_as_is_single_string() {
        let matches = ArgMatches::new();
        let mut runner = Runner::new(&matches);
        let mut repofile = RepoFile::new();
        // single string should not be valid for include_as
        repofile.include_as = Some(vec!["dsadsa".into()]);
        repofile.repo_name = Some("dsadsa".into());
        runner.repo_file = repofile;
        runner.validate_repo_file();
    }
}
