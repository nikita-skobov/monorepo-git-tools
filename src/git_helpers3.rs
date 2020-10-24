/// For the v3 version I rewrote the git_helpers module to interface
/// with git via the CLI instead of libgit2

use super::exec_helpers;

pub fn pull(
    remote_name: &str,
    remote_branch_name: Option<&str>,
    num_commits: Option<u32>,
) -> Result<(), String> {
    let mut exec_args = vec![
        "git", "pull",
        remote_name,
        remote_branch_name.unwrap_or("HEAD"),
    ];

    let mut _depth_string = String::from("");
    if let Some(n) = num_commits {
        _depth_string = format!("--depth={}", n);
        exec_args.push(_depth_string.as_str());
    }

    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

/// target is the current branch
pub fn merge_branch(
    source_branch: &str,
) -> Result<(), String> {
    let exec_args = vec![
        "git", "merge",
        source_branch
    ];
    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

pub fn make_orphan_branch_and_checkout(
    orphan_branch_name: &str
) -> Result<(), String> {
    let exec_args = vec![
        "git", "checkout",
        "--orphan", orphan_branch_name,
    ];
    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

/// after checking out an orphan branch, gits index
/// will be full of files that exist on the filesystem,
/// and git says they are ready to be added. We want
/// to tell git to delete these files (which is safe to do because
/// they exist in another branch)
pub fn remove_index_and_files() -> Result<(), String> {
    let exec_args = ["git", "rm", "-rf", "."];
    let success = exec_helpers::executed_successfully(&exec_args);
    match success {
        true => Ok(()),
        false => Err("Failed to git rm -rf .".into()),
    }
}

pub fn branch_exists(branch_name: &str) -> bool {
    let branch_ref = format!("refs/heads/{}", branch_name);
    let exec_args = [
        "git", "show-ref", "--verify", "--quiet", branch_ref.as_str()
    ];
    // will return 0 (true) if branch exists , 1 (false) otherwise
    exec_helpers::executed_successfully(&exec_args)
}

pub fn delete_branch(
    branch_name: &str
) -> Result<(), String> {
    let exec_args = [
        "git", "branch", "-D", branch_name,
    ];
    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

pub fn checkout_branch(
    branch_name: &str,
    make_new: bool,
) -> Result<(), String> {
    let mut exec_args = vec![
        "git", "checkout"
    ];
    if make_new {
        exec_args.push("-b");
        exec_args.push(branch_name);
    } else {
        exec_args.push(branch_name);
    }

    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}
