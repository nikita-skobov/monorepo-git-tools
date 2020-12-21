use super::split::panic_if_array_invalid;
use super::split::try_get_repo_name_from_remote_repo;
use super::repo_file::RepoFile;
use super::repo_file;
use super::die;
use super::core;
use super::cli::MgtCommandSplit;

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

pub fn run_split_out(
    cmd: &mut MgtCommandSplit,
) {
    let repo_file_path = if cmd.repo_file.len() < 1 {
        die!("Must provide repo path argument");
    } else {
        cmd.repo_file[0].clone()
    };

    let repo_file = repo_file::parse_repo_file_from_toml_path(&repo_file_path);
    run_split_out_from_repo_file(cmd, repo_file)
}

pub fn run_split_out_as(
    cmd: &mut MgtCommandSplit
) {
    let include_as_src = match cmd.as_subdir {
        Some(ref s) => s,
        None => die!("Must provide an --as <subdirectory> option"),
    };
    let output_branch = match cmd.output_branch {
        Some(ref s) => s,
        None => die!("Must provide an --output-branch <branch_name> when doing split-out-as"),
    };
    let mut repo_file = RepoFile::new();
    repo_file.include_as = Some(vec![
        include_as_src.into(), " ".into(),
    ]);
    repo_file.repo_name = Some(output_branch.into());
    run_split_out_from_repo_file(cmd, repo_file)
}

pub fn run_split_out_from_repo_file(
    cmd: &mut MgtCommandSplit,
    repo_file: RepoFile,
) {
    let mut repo_file = repo_file;
    core::verify_dependencies();
    validate_repo_file(&mut repo_file, &mut cmd.output_branch);
    core::go_to_repo_root();
    core::safe_to_proceed();
    let arg_strings = generate_arg_strings(&repo_file, cmd.verbose);
    core::make_and_checkout_output_branch(
        &cmd.output_branch,
        cmd.dry_run,
        cmd.verbose,
    );

    let log_p = if cmd.dry_run { "   # " } else { "" };
    if let Some(ref b) = cmd.output_branch {
        println!("{}Running filter commands on temporary branch: {}", log_p, b);
    }

    arg_strings.filter(
        &cmd.output_branch,
        cmd.dry_run,
        cmd.verbose
    );

    // for split out, rebase is a bit different because
    // we actually need to fetch the remote repo|branch that
    // the user specified in the repo file, and then checkout to that branch
    // then save its ref, then checkout back to the newly created branch,
    // then run rebase, then delete the fetched branch since it is not
    // useful to us anymore after the rebase
    let runner_should_rebase = cmd.rebase.is_some(); // runner.should_rebase
    let runner_should_topbase = cmd.topbase.is_some(); // runner.should_topbase
    let either_rebase_or_topbase = runner_should_rebase || runner_should_topbase;
    if either_rebase_or_topbase {
        // TODO: what if user has a branch with this name...
        let tmp_remote_branch = "mgt-remote-branch-tmp";
        core::make_and_checkout_orphan_branch(tmp_remote_branch, cmd.dry_run, cmd.verbose);

        let remote_branch: Option<&str> = match &repo_file.remote_branch {
            Some(branch_name) => Some(branch_name.as_str()),
            None => None,
        };
        // if user provided a remote_branch name
        // on the command line, let that override what
        // is present in the repo file:
        let remote_branch = match get_remote_branch_from_args(cmd) {
            None => remote_branch,
            Some(new_remote_branch) => Some(new_remote_branch.as_str()),
        };

        core::populate_empty_branch_with_remote_commits(
            &repo_file,
            cmd.input_branch.as_deref(),
            remote_branch,
            cmd.num_commits,
            cmd.dry_run
        );
        let current_ref = core::get_current_ref();

        core::checkout_output_branch(
            cmd.output_branch.clone(),
            cmd.dry_run,
            cmd.verbose
        );

        let res = if runner_should_rebase {
            println!("{}Rebasing", log_p);
            let res = core::rebase(current_ref, cmd.dry_run, cmd.verbose);
            core::delete_branch(tmp_remote_branch);
            res
        } else if runner_should_topbase {
            use super::topbase;
            println!("{}Topbasing", log_p);
            let should_add_branch_label = true;
            let res = topbase::topbase(
                cmd.output_branch.clone().unwrap(),
                tmp_remote_branch.to_string(),
                cmd.dry_run,
                cmd.verbose,
                should_add_branch_label,
            );
            core::delete_branch(tmp_remote_branch);
            res
        } else {
            Ok(())
        };

        if let Ok(_) = res {
            println!("{}Success!", log_p);
        } else if let Err(e) = res {
            die!("{}", e);
        }
    }
}


pub fn validate_repo_file(
    repo_file: &mut RepoFile,
    output_branch: &mut Option<String>,
) {
    let missing_output_branch = output_branch.is_none();
    let missing_repo_name = repo_file.repo_name.is_none();
    let missing_remote_repo = repo_file.remote_repo.is_none();
    let missing_include_as = repo_file.include_as.is_none();
    let missing_include = repo_file.include.is_none();

    if missing_remote_repo && missing_repo_name && missing_output_branch {
        die!("Must provide either repo_name or remote_repo in your repofile");
    }

    if missing_include && missing_include_as {
        die!("Must provide either include or include_as in your repofile");
    }

    if missing_output_branch && missing_repo_name && !missing_remote_repo {
        let output_branch_str = try_get_repo_name_from_remote_repo(
            repo_file.remote_repo.clone().unwrap()
        );
        repo_file.repo_name = Some(output_branch_str.clone());
        *output_branch = Some(output_branch_str);
    } else if missing_output_branch && ! missing_repo_name {
        // make the repo_name the output branch name
        *output_branch = Some(repo_file.repo_name.clone().unwrap());
    }

    panic_if_array_invalid(&repo_file.include, true, "include");
    panic_if_array_invalid(&repo_file.include_as, false, "include_as");
}


fn generate_arg_strings(
    repo_file: &RepoFile,
    verbose: bool,
) -> core::ArgStrings {
    let include_arg_str = generate_split_out_arg_include(repo_file);
    let include_as_arg_str = generate_split_out_arg_include_as(repo_file);
    let exclude_arg_str = generate_split_out_arg_exclude(repo_file);

    let log_p = if verbose { " #  " } else { "" };
    if verbose {
        println!("{}include_arg_str: {}", log_p, include_arg_str.join(" "));
        println!("{}include_as_arg_str: {}", log_p, include_as_arg_str.join(" "));
        println!("{}exclude_arg_str: {}", log_p, exclude_arg_str.join(" "));
    }

    let include = if include_arg_str.len() != 0 {
        Some(include_arg_str)
    } else { None };

    let include_as = if include_as_arg_str.len() != 0 {
        Some(include_as_arg_str)
    } else { None };

    let exclude = if exclude_arg_str.len() != 0 {
        Some(exclude_arg_str)
    } else { None };


    core::ArgStrings {
        include_as,
        include,
        exclude,
    }
}

pub fn get_remote_branch_from_args(
    cmd: &MgtCommandSplit,
) -> Option<&String> {
    if cmd.topbase.is_none() && cmd.rebase.is_none() {
        return None;
    }

    if let Some(ref s) = cmd.topbase {
        if *s == "" {
            return None;
        } else {
            return Some(s);
        }
    }
    if let Some(ref s) = cmd.rebase {
        if *s == "" {
            return None;
        } else {
            return Some(s);
        }
    }

    // should never get here but whatever
    return None;
}


#[cfg(test)]
mod test {
    use super::*;

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
}
