use super::core;
use super::repo_file::RepoFile;
use super::repo_file;
use super::die;
use super::verify;
use super::git_helpers3;
use super::cli::MgtCommandSplit;

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
    let filter_rules = generate_gitfilter_filterrules(&repo_file, cmd.verbose);
    core::make_and_checkout_output_branch(
        &cmd.output_branch,
        cmd.dry_run,
        cmd.verbose,
    );

    let log_p = if cmd.dry_run { "   # " } else { "" };
    if let Some(ref b) = cmd.output_branch {
        println!("{}Running filter commands on temporary branch: {}", log_p, b);
    }

    let output_branch = match &cmd.output_branch {
        Some(o) => o.clone(),
        None => die!("Failed to find output branch"),
    };
    core::perform_gitfilter(filter_rules, output_branch, cmd.dry_run, cmd.verbose);

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
        let output_branch_str = core::try_get_repo_name_from_remote_repo(
            repo_file.remote_repo.clone().unwrap()
        );
        repo_file.repo_name = Some(output_branch_str.clone());
        *output_branch = Some(output_branch_str);
    } else if missing_output_branch && ! missing_repo_name {
        // make the repo_name the output branch name
        *output_branch = Some(repo_file.repo_name.clone().unwrap());
    }

    core::panic_if_array_invalid(&repo_file.include, true, "include");
    core::panic_if_array_invalid(&repo_file.include_as, false, "include_as");
}

fn generate_gitfilter_filterrules(
    repo_file: &RepoFile,
    verbose: bool,
) -> gitfilter::filter::FilterRules {
    let mut file_ops = verify::get_vec_of_file_ops(&repo_file);
    let filter_rules = verify::make_filter_rules(&mut file_ops);
    filter_rules
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
