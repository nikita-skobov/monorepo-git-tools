use std::convert::From;
use std::fs;
use std::path::Path;
use std::fmt::Display;
use std::{collections::HashSet, path::PathBuf};

use super::split_out;
use super::git_helpers3;
use super::exec_helpers;
use super::repo_file::RepoFile;
use super::repo_file::generate_repo_file_toml;
use super::die;
use super::repo_file;
use super::cli::MgtCommandSplit;
use super::core;
use super::verify;
use super::topbase;


pub fn run_split_in(cmd: &mut MgtCommandSplit) {
    let repo_file_path = if cmd.repo_file.len() < 1 {
        die!("Must provide repo path argument");
    } else {
        cmd.repo_file[0].clone()
    };

    let repo_file = repo_file::parse_repo_file_from_toml_path(&repo_file_path);
    let is_split_in_as = false;
    run_split_in_from_repo_file(cmd, repo_file, is_split_in_as)
}

pub fn run_split_in_as(cmd: &mut MgtCommandSplit) {
    let include_as_src = match cmd.as_subdir {
        Some(ref s) => s,
        None => die!("Must provide an --as <subdirectory> option"),
    };
    // the field is called repo_file, but in split-in-as
    // its actually the repo_uri
    let repo_uri = match cmd.repo_file.len() {
        0 => die!("Must provide a git-repo-uri for split-in-as"),
        _ => cmd.repo_file[0].clone(),
    };
    let mut repo_file = RepoFile::new();
    repo_file.include_as = Some(vec![
        include_as_src.into(), " ".into(),
    ]);
    repo_file.remote_repo = Some(repo_uri.into());
    let is_split_in_as = true;
    run_split_in_from_repo_file(cmd, repo_file, is_split_in_as);
}

pub fn run_split_in_from_repo_file(
    cmd: &mut MgtCommandSplit,
    repo_file: RepoFile,
    split_in_as: bool,
) {
    let mut repo_file = repo_file;
    core::verify_dependencies();
    validate_repo_file(cmd, &mut repo_file);
    core::go_to_repo_root();
    core::safe_to_proceed();
    let current_ref = core::get_current_ref();

    let orphan_branch_name = match cmd.output_branch {
        Some(ref s) => s,
        None => die!("Failed to parse a valid output branch. you may alternatively provide one with --output-branch <branch_name>"),
    };

    core::make_and_checkout_orphan_branch(
        orphan_branch_name,
        cmd.dry_run,
        cmd.verbose,
    );

    let remote_branch: Option<&str> = match &repo_file.remote_branch {
        Some(branch_name) => Some(branch_name.as_str()),
        None => None,
    };
    // if user provided a remote_branch name
    // on the command line, let that override what
    // is present in the repo file:
    let remote_branch = match split_out::get_remote_branch_from_args(cmd) {
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

    let log_p = if cmd.dry_run { "   # " } else { "" };
    if let Some(ref b) = cmd.output_branch {
        println!("{}Running filter commands on temporary branch: {}", log_p, b);
    }
    
    let filter_rules = generate_gitfilter_filterrules(&repo_file, cmd.verbose);
    core::perform_gitfilter(filter_rules, orphan_branch_name.clone(), cmd.dry_run, cmd.verbose);
    let res = if cmd.topbase.is_some() {
        println!("{}Topbasing", log_p);
        let should_add_branch_label = false;
        topbase::topbase(
            cmd.output_branch.clone().unwrap(),
            current_ref.unwrap(),
            cmd.dry_run,
            cmd.verbose,
            should_add_branch_label,
        )
    } else if cmd.rebase.is_some() {
        println!("{}Rebasing", log_p);
        core::rebase(current_ref, cmd.dry_run, cmd.verbose)
    } else {
        Ok(())
    };

    if let Err(e) = res {
        die!("{}", e);
    }

    // only allow repo file generation for split-in-as
    // subcommand. split-in already has a repo file...
    if split_in_as && cmd.generate_repo_file {
        let repo_file_name = match cmd.output_branch {
            Some(ref n) => n,
            None => "meta",
        };
        match generate_repo_file(repo_file_name, &repo_file) {
            Err(e) => die!("Failed to generate repo file: {}", e),
            Ok(_) => (),
        }
    }

    println!("{}Success!", log_p);
}

fn generate_gitfilter_filterrules(
    repo_file: &RepoFile,
    verbose: bool,
) -> gitfilter::filter::FilterRules {
    let mut file_ops = verify::get_vec_of_file_ops_with_order(&repo_file, false);
    let filter_rules = verify::make_filter_rules(&mut file_ops);
    filter_rules
}

pub fn generate_repo_file(
    repo_name: &str,
    repofile: &RepoFile
) -> Result<(), String> {
    let repo_file_path_str = format!("{}.rf", repo_name);
    let repo_file_path = std::path::PathBuf::from(&repo_file_path_str);
    if repo_file_path.exists() {
        let err_str = format!("{}.rf already exists", repo_name);
        return Err(err_str);
    }

    let repo_file_str = generate_repo_file_toml(repofile);
    match std::fs::write(repo_file_path_str, repo_file_str) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn validate_repo_file(
    cmd: &mut MgtCommandSplit,
    repo_file: &mut RepoFile,
) {
    let input_branch = match cmd.input_branch {
        None => None,
        Some(ref branch_name) => {
            if ! git_helpers3::branch_exists(&branch_name) {
                die!("You specified an input branch of {}, but that branch was not found", branch_name);
            }
            Some(branch_name.clone())
        },
    };

    let missing_output_branch = cmd.output_branch.is_none();
    let missing_input_branch = cmd.input_branch.is_none();
    let missing_repo_name = repo_file.repo_name.is_none();
    let missing_remote_repo = repo_file.remote_repo.is_none();
    let missing_include_as = repo_file.include_as.is_none();
    let missing_include = repo_file.include.is_none();

    if missing_remote_repo && missing_input_branch && ! missing_output_branch {
        die!("Must provide either repo_name in your repofile, or specify a --input-branch argument");
    }

    if missing_include && missing_include_as {
        die!("Must provide either include or include_as in your repofile");
    }

    if missing_repo_name && !missing_remote_repo && missing_output_branch {
        let output_branch_str = core::try_get_repo_name_from_remote_repo(
            repo_file.remote_repo.clone().unwrap()
        );
        repo_file.repo_name = Some(output_branch_str.clone());
        cmd.output_branch = Some(output_branch_str);
    } else if missing_output_branch && ! missing_repo_name {
        // make the repo_name the output branch name
        cmd.output_branch = Some(repo_file.repo_name.clone().unwrap());
    } else if missing_output_branch && ! missing_input_branch {
        // make the output_branch the name of the input_branch -reverse
        let output_branch_str = format!("{}-reverse", input_branch.clone().unwrap());
        cmd.output_branch = Some(output_branch_str);
    }

    core::panic_if_array_invalid(&repo_file.include, true, "include");
    core::panic_if_array_invalid(&repo_file.include_as, false, "include_as");
}
