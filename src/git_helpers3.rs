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
