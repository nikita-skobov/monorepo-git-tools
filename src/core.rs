use std::env;
use die::die;
use std::path::PathBuf;
use std::path::MAIN_SEPARATOR;

use git_url_parse::GitUrl;

use super::exec_helpers;
use super::git_helpers3;
use super::repo_file::RepoFile;
use super::cli::Direction;

/// argument strings to be executed when
/// running git-filter-repo
pub struct ArgStrings {
    pub include: Option<Vec<String>>,
    pub include_as: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

impl ArgStrings {
    pub fn filter_out(
        &self,
        output_branch: &Option<String>,
        dry_run: bool,
        verbose: bool,
    ) {
        filter_all_arg_strs(
            self,
            Direction::Out,
            output_branch,
            dry_run,
            verbose,
        )
    }
    pub fn filter_in(
        &self,
        output_branch: &Option<String>,
        dry_run: bool,
        verbose: bool,
    ) {
        filter_all_arg_strs(
            self,
            Direction::In,
            output_branch,
            dry_run,
            verbose,
        )
    }
}

pub fn get_current_ref() -> Option<String> {
    match git_helpers3::get_current_ref() {
        Ok(s) => Some(s),
        Err(_) => None,
    }
}

pub fn _get_current_dir() -> PathBuf {
    match env::current_dir() {
        Ok(pathbuf) => pathbuf,
        Err(_) => die!("Failed to find your current directory"),
    }
}

pub fn get_repo_root() -> PathBuf {
    let repo_path = match git_helpers3::get_repo_root() {
        Ok(p) => p,
        Err(_) => die!("Must run this command from a git repository"),
    };

    PathBuf::from(repo_path)
}

pub fn delete_branch(branch_name: &str) {
    if let Err(e) = git_helpers3::delete_branch(branch_name) {
        println!("Failed to delete branch: {}. {}", branch_name, e);
    }
}

pub fn go_to_repo_root() {
    let repo_root = get_repo_root();
    if let Err(e) = env::set_current_dir(repo_root) {
        die!("Failed to change to repo root: {}", e);
    }
}

pub fn checkout_output_branch(
    output_branch: Option<String>,
    dry_run: bool,
    verbose: bool,
) {
    let output_branch_name = output_branch.unwrap();
    if dry_run {
        println!("git checkout {}", output_branch_name);
        return;
    }

    if let Err(e) = git_helpers3::checkout_branch(
        output_branch_name.as_str(),
        false,
    ) {
        die!("Failed to checkout branch {}", e);
    }

    if verbose {
        let log_p = if dry_run { "   # " } else { "" };
        println!("{} checked out branch {}", log_p, output_branch_name);
    }
}

pub fn rebase(
    repo_original_ref: Option<String>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), String> {
    let upstream_branch = match repo_original_ref {
        Some(ref branch) => branch,
        None => {
            println!("Failed to get repo original ref. Not going to rebase");
            return Ok(());
        }
    };

    let upstream_branch = upstream_branch.replace("refs/heads/", "");

    if verbose {
        println!("rebasing onto {}", upstream_branch);
    }
    if dry_run {
        // since we are already on the rebase_from_branch
        // we dont need to specify that in the git command
        // the below command implies: apply rebased changes in
        // the branch we are already on
        println!("git rebase {}", upstream_branch);
        return Ok(());
    }

    let args = [
        "git", "rebase", upstream_branch.as_str(),
    ];
    let err_msg = match exec_helpers::execute(&args) {
        Err(e) => Some(vec![format!("{}", e)]),
        Ok(o) => {
            match o.status {
                0 => None,
                _ => Some(vec![o.stderr.lines().next().unwrap().to_string()]),
            }
        },
    };
    if let Some(err) = err_msg {
        let err_details = match verbose {
            true => format!("{}", err.join("\n")),
            false => "".into(),
        };
        let err_details = format!("Failed to rebase\n{}", err_details);
        return Err(err_details);
    }

    Ok(())
}

/// panic if all dependencies are not met
pub fn verify_dependencies() {
    if ! exec_helpers::executed_successfully(&["git", "--version"]) {
        die!("Failed to run. Missing dependency 'git'");
    }
    if ! exec_helpers::executed_successfully(&["git", "filter-repo", "--version"]) {
        die!("Failed to run. Missing dependency 'git-filter-repo'");
    }
}

/// check the state of the git repository. exit if
/// there are modified files, in the middle of a merge conflict
/// etc...
pub fn safe_to_proceed() {
    // TODO: also check for other things like:
    // are there files staged? are we resolving a conflict?
    // im just too lazy right now, and this is the most likely scenario
    let args = ["git", "ls-files", "--modified"];
    let output = match exec_helpers::execute(&args) {
        Ok(o) => match o.status {
            0 => o.stdout,
            _ => die!("Failed to run ls-files: {}", o.stderr),
        },
        Err(e) => die!("Failed to run ls-files: {}", e),
    };
    if ! output.is_empty() {
        die!("You have modified changes. Please stash or commit your changes before running this command");
    }
}

pub fn make_and_checkout_output_branch(
    output_branch: &Option<String>,
    dry_run: bool,
    verbose: bool,
) {
    let output_branch_name = match output_branch {
        Some(s) => s,
        None => die!("Must provide an output branch"),
    };

    if dry_run {
        println!("git checkout -b {}", output_branch_name);
        return;
    }

    if git_helpers3::checkout_branch(
        output_branch_name.as_str(),
        true,
    ).is_err() {
        die!("Failed to checkout new branch");
    }

    if verbose {
        println!("created and checked out new branch {}", output_branch_name);
    }
}

pub fn make_and_checkout_orphan_branch(
    orphan_branch: &str,
    dry_run: bool,
    verbose: bool,
) {
    if dry_run {
        println!("git checkout --orphan {}", orphan_branch);
        println!("git rm -rf . > /dev/null");
        return;
    }

    if git_helpers3::make_orphan_branch_and_checkout(
        orphan_branch,
    ).is_err() {
        die!("Failed to checkout orphan branch");
    }

    // on a new orphan branch our existing files appear in the stage
    // we need to do "git rm -rf ."
    // the 'dot' should be safe to do as long as
    // we are in the root of the repository, but this method
    // should only be called after we cd into the root
    if git_helpers3::remove_index_and_files().is_err() {
        die!("Failed to remove git indexed files after making orphan");
    }
    if verbose {
        println!("created and checked out orphan branch {}", orphan_branch);
    }
}

pub fn populate_empty_branch_with_remote_commits(
    repo_file: &RepoFile,
    input_branch: Option<&str>,
    remote_branch: Option<&str>,
    num_commits: Option<u32>,
    dry_run: bool,
) {
    let remote_repo = repo_file.remote_repo.clone();
    let log_p = if dry_run { "   # " } else { "" };

    match (dry_run, input_branch) {
        (true, Some(branch_name)) => println!("git merge {}", branch_name),
        (true, None) => println!("git pull {}", remote_repo.unwrap()),
        (false, Some(branch_name)) => {
            println!("{}Merging {}", log_p, branch_name);
            let _ = git_helpers3::merge_branch(&branch_name[..]);
        },
        (false, None) => {
            let remote_repo_name = remote_repo.clone().unwrap_or("?".into());
            let remote_branch_name = remote_branch.clone().unwrap_or("".into());
            let remote_string = if remote_branch_name != "" {
                format!("{}:{}", remote_repo_name, remote_branch_name)
            } else { format!("{}", remote_repo_name) };
            println!("{}Pulling from {}", log_p, remote_string);
            if git_helpers3::pull(
                &remote_repo.unwrap()[..],
                remote_branch,
                num_commits
            ).is_err() {
                die!("Failed to pull remote repo {}", remote_string);
            }
        },
    };
}

pub fn filter_all_arg_strs(
    arg_strs: &ArgStrings,
    direction: Direction,
    output_branch: &Option<String>,
    dry_run: bool,
    verbose: bool,
) {
    if let Direction::Out = direction {
        filter_from_arg_str(
            &arg_strs.include,
            output_branch,
            dry_run,
            verbose,
            "Filtering include",
        );
        filter_from_arg_str(
            &arg_strs.exclude,
            output_branch,
            dry_run,
            verbose,
            "Filtering exclude",
        );
        filter_from_arg_str(
            &arg_strs.include_as,
            output_branch,
            dry_run,
            verbose,
            "Filtering include_as",
        );
    } else if let Direction::In = direction {
        filter_from_arg_str(
            &arg_strs.exclude,
            output_branch,
            dry_run,
            verbose,
            "Filtering exclude",
        );
        filter_from_arg_str(
            &arg_strs.include_as,
            output_branch,
            dry_run,
            verbose,
            "Filtering include_as",
        );
        filter_from_arg_str(
            &arg_strs.include,
            output_branch,
            dry_run,
            verbose,
            "Filtering include",
        );
    }
}

pub fn filter_from_arg_str(
    arg_str: &Option<Vec<String>>,
    output_branch: &Option<String>,
    dry_run: bool,
    verbose: bool,
    verbose_log: &str,
) {
    if arg_str.is_none() {
        // dont run filter if this arg was not provided
        return;
    }

    let output_branch_name = match output_branch {
        Some(s) => s,
        None => die!("Failed to get output branch"),
    };

    if let Some(ref s) = arg_str {
        let arg_str_opt = s.clone();
        let unambiguous_output_branch = format!("refs/heads/{}", output_branch_name);
        let arg_vec = generate_filter_arg_vec(
            &arg_str_opt,
            unambiguous_output_branch.as_str(),
        );

        run_filter(arg_vec, verbose_log, dry_run, verbose)
    }
}

pub fn run_filter(
    arg_vec: Vec<&str>,
    verbose_log: &str,
    dry_run: bool,
    verbose: bool,
) {
    if dry_run {
        println!("{}", arg_vec.join(" "));
        return;
    }
    if verbose {
        println!("{}", verbose_log);
    }
    let err_msg = match exec_helpers::execute(&arg_vec) {
        Ok(o) => match o.status {
            0 => None,
            _ => Some(o.stderr),
        },
        Err(e) => Some(format!("{}", e)),
    };
    if let Some(err) = err_msg {
        die!("Failed to execute: \"{}\"\n{}", arg_vec.join(" "), err);
    }
}

pub fn generate_filter_arg_vec<'a>(
    args: &'a Vec<String>,
    output_branch: &'a str,
) -> Vec<&'a str> {
    let mut arg_vec = vec!["git", "filter-repo"];
    for arg in args {
        arg_vec.push(arg);
    }
    arg_vec.push("--refs");
    arg_vec.push(&output_branch);
    arg_vec.push("--force");

    arg_vec
}

pub fn panic_if_array_invalid(var: &Option<Vec<String>>, can_be_single: bool, varname: &str) {
    match var {
        Some(v) => {
            if ! include_var_valid(&v, can_be_single) {
                die!("{} is invalid. Must be either a single string, or an even length array of strings", varname);
            }
        },
        _ => (),
    };
}

// works for include, or include_as
// the variable is valid if it is a single item,
// or if it is multiple items, it is valid if it has an even length
pub fn include_var_valid(var: &Vec<String>, can_be_single: bool) -> bool {
    let vlen = var.len();
    if vlen == 1 && can_be_single {
        return true;
    }
    if vlen >= 1 && vlen % 2 == 0 {
        return true;
    }
    return false;
}

pub fn try_get_repo_name_with_slash_type(remote_repo: &String, slash_type: char) -> String {
    let mut out_str = remote_repo.clone().trim_end().to_string();
    if !is_valid_remote_repo(&remote_repo) {
        out_str = "".into();
    }
    if out_str.ends_with(slash_type) {
        out_str.pop();
    }
    if !out_str.contains(slash_type) {
        out_str = "".into();
    }
    out_str = get_string_after_last_slash(out_str, slash_type);
    out_str = get_string_before_first_dot(out_str);

    return out_str;
}

pub fn is_valid_remote_repo(remote_repo: &String) -> bool {
    GitUrl::parse(remote_repo).is_ok()
}

// try to parse the remote repo
pub fn try_get_repo_name_from_remote_repo(remote_repo: String) -> String {
    let slash_type = MAIN_SEPARATOR;
    let next_slash_type = if slash_type == '/' { '\\' } else { '/' };

    // try to use native slash first:
    let mut repo_name = try_get_repo_name_with_slash_type(&remote_repo, slash_type);
    if repo_name == "" {
        repo_name = try_get_repo_name_with_slash_type(&remote_repo, next_slash_type);
    }

    if repo_name == "" {
        die!("Failed to parse repo_name from remote_repo: {}", remote_repo);
    }

    repo_name
}


fn get_string_after_last_slash(s: String, slash_type: char) -> String {
    let mut pieces = s.rsplit(slash_type);
    match pieces.next() {
        Some(p) => p.into(),
        None => s.into(),
    }
}

fn get_string_before_first_dot(s: String) -> String {
    let mut pieces = s.split('.');
    match pieces.next() {
        Some(p) => p.into(),
        None => s.into(),
    }
}
