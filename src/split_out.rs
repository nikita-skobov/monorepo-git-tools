use clap::ArgMatches;

use super::split::panic_if_array_invalid;
use super::split::Runner;
use super::split::try_get_repo_name_from_remote_repo;
use super::repo_file::RepoFile;
use super::git_helpers;

pub trait SplitOut {
    fn validate_repo_file(self) -> Self;
    fn generate_arg_strings(self) -> Self;
    fn make_and_checkout_output_branch(self) -> Self;
}

impl<'a> SplitOut for Runner<'a> {
    fn validate_repo_file(mut self) -> Self {
        let missing_output_branch = self.output_branch.is_none();
        let missing_repo_name = self.repo_file.repo_name.is_none();
        let missing_remote_repo = self.repo_file.remote_repo.is_none();
        let missing_include_as = self.repo_file.include_as.is_none();
        let missing_include = self.repo_file.include.is_none();
    
        if missing_remote_repo && missing_repo_name {
            panic!("Must provide either repo_name or remote_repo in your repofile");
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

    fn make_and_checkout_output_branch(self) -> Self {
        let output_branch_name = self.output_branch.clone().unwrap();
        if self.dry_run {
            println!("git checkout -b {}", output_branch_name);
            return self;
        }

        match self.repo {
            Some(ref r) => {
                let success = git_helpers::make_new_branch_from_head_and_checkout(
                    r,
                    output_branch_name.as_str(),
                ).is_ok();
                if ! success {
                    panic!("Failed to checkout new branch");
                }
            },
            _ => panic!("Something went horribly wrong!"),
        };
        if self.verbose {
            println!("{}created and checked out new branch {}", self.log_p, output_branch_name);
        }

        self
    }
}


// iterate over both the include, and include_as
// repofile variables, and generate an overall
// include string that can be passed to
// git-filter-repo
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

    // sources are the even indexed elements, dest are the odd
    let sources = include_as.iter().skip(0).step_by(2);
    let destinations = include_as.iter().skip(1).step_by(2);
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

pub fn run_split_out(matches: &ArgMatches) {
    Runner::new(matches)
        .get_repo_file()
        .verify_dependencies()
        .validate_repo_file()
        .save_current_dir()
        .get_repository_from_current_dir()
        .change_to_repo_root()
        .generate_arg_strings()
        .make_and_checkout_output_branch()
        .filter_include()
        .filter_exclude()
        .filter_include_as();
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
        assert_eq!(filter_args, "--path 123 --path abc --path xyz");
    }

    #[test]
    fn should_generate_exclude_args_properly_for_one_exclude() {
        let mut repofile = RepoFile::new();
        repofile.exclude = Some(vec![
            "one".into(),
        ]);
        let filter_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_args, "--invert-paths --path one");
    }

    #[test]
    fn should_generate_exclude_args_properly_for_multiple_exclude() {
        let mut repofile = RepoFile::new();
        repofile.exclude = Some(vec![
            "one".into(), "two".into(), "three".into(),
        ]);
        let filter_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_args, "--invert-paths --path one --path two --path three");
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
    fn should_generate_empty_strings_for_generating_args_of_none() {
        let mut repofile = RepoFile::new();
        repofile.exclude = None;
        repofile.include = None;
        repofile.include_as = None;

        let filter_exclude_args = generate_split_out_arg_exclude(&repofile);
        assert_eq!(filter_exclude_args, "");

        let filter_include_args = generate_split_out_arg_include(&repofile);
        assert_eq!(filter_include_args, "");

        let filter_include_as_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_include_as_args, "");
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
        assert_eq!(filter_args, "--path-rename abc-src:abc-dest --path-rename xyz-src:xyz-dest");
    }

    // if include_as is None, it shouldnt fail, but rather
    // just return an empty string
    #[test]
    fn gen_split_out_arg_include_as_should_not_fail_if_no_include_as() {
        let repofile = RepoFile::new();
        let filter_args = generate_split_out_arg_include_as(&repofile);
        assert_eq!(filter_args, "");
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
