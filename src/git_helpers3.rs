/// For the v3 version I rewrote the git_helpers module to interface
/// with git via the CLI instead of libgit2

use super::exec_helpers;
use std::{io::{self, BufReader}, io::BufRead, process::Stdio};
use crate::{ioerre, ioerr};
pub use crate::blob_log_parser::*;

#[derive(Debug, Clone)]
pub struct Oid {
    pub hash: String,
}
impl Oid {
    /// it is assumed that short() will not be called
    /// on an empty oid
    pub fn short(&self) -> &str {
        let substr = self.hash.get(0..7);
        substr.unwrap()
    }
    pub fn long(&self) -> &String {
        &self.hash
    }
}
#[derive(Debug, Clone)]
pub struct Commit {
    pub id: Oid,
    pub summary: String,
    pub is_merge: bool,
}

impl Commit {
    pub fn new(hash: &str, summary: String, is_merge: bool) -> Commit {
        let oid = Oid { hash: hash.to_string() };
        Commit { id: oid, summary, is_merge }
    }
}

/// iterates a list of commits and parses
/// the blob summary of each commit and then passes the commit
/// and blobs to a callback. The callback function returns true if
/// it wants to be done reading from the stream, in which case
/// this function will stop reading from the stream and kill the process.
/// Optionally pass in a number of commits to read including the first
/// one indicated by committish. (this corresponds to git log [...] -n <number-of-commits>)
pub fn iterate_blob_log<T>(
    committish: &str,
    num_commits: Option<usize>,
    callback: T,
) -> io::Result<()>
    where T: FnMut(CommitWithBlobs) -> bool,
{
    // TODO: add the '-m' flag if we want to see merge commits with a full blob diff
    // by default, merge commits do not have a blob summary, which
    // makes it easy to tell which commits are merges or not. this default
    // is desirable 9 times out of 10. not sure when -m would be desired though.
    let mut exec_args = vec![
        "git", "--no-pager", "log", "--no-color", "--raw",
        "--pretty=oneline", committish,
    ];
    let n_str = match num_commits {
        Some(n) => n.to_string(),
        None => "".to_string()
    };
    if ! n_str.is_empty() {
        exec_args.push("-n");
        exec_args.push(&n_str);
    }

    let mut child = exec_helpers::spawn_with_env_ex(
        &exec_args,
        &[], &[],
        Some(Stdio::null()), Some(Stdio::null()), Some(Stdio::piped()),
    )?;

    let stdout = child.stdout.as_mut()
        .ok_or(ioerr!("Failed to get child stdout for reading git log of {}", committish))?;
    let mut stdout_read = BufReader::new(stdout);

    let output = iterate_blob_log_from_lines(&mut stdout_read, callback);
    let (should_kill_child, output) = match output {
        Ok(o) => (o, Ok(o)),
        // if there was an error parsing the blob log lines,
        // we should kill the child just in case to prevent
        // running forever on child.wait()
        Err(e) => (true, Err(e))
    };

    if should_kill_child {
        let _ = child.kill();
    } else {
        // only return this child.wait() error if
        // our output response is ok. if our output is an error,
        // then we would rather return that error instead of an error
        // that came from calling child.wait()
        let child_wait_res = child.wait();
        if output.is_ok() {
            let child_res = child_wait_res?;
            // return an error if the child exited with error:
            if ! child_res.success() {
                return ioerre!("git log --raw --oneline -m {} exited unsuccessfully", committish);
            }
        }
    }

    if let Err(e) = output {
        return Err(e);
    }

    Ok(())
}

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

pub fn get_current_ref() -> Result<String, String> {
    let exec_args = [
        "git", "rev-parse", "--abbrev-ref", "HEAD"
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                // dont want trailing new line
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_all_commits_from_ref(
    refname: &str,
    num_commits: Option<usize>,
) -> Result<Vec<Commit>, String> {
    // TODO: in the future might want more info than
    // just the hash and summary
    let mut exec_args = vec![
        "git", "log", refname, "--format=%H [%p] %s",
    ];
    let mut n_str = "".to_string();
    if let Some(n) = num_commits {
        n_str = n.to_string();
    }
    if ! n_str.is_empty() {
        exec_args.push("-n");
        exec_args.push(&n_str);
    }
    let mut commits = vec![];
    let out_str = match exec_helpers::execute(&exec_args) {
        Err(e) => return Err(e.to_string()),
        Ok(out) => match out.status {
            0 => out.stdout,
            _ => return Err(out.stderr),
        }
    };

    for line in out_str.lines() {
        // everything before first space is
        // the commit hash. everything after is the summary
        let mut line_split = line.split(" ");
        let hash = line_split.nth(0);
        let hash = if let Some(h) = hash {
            h.to_string()
        } else {
            return Err("Failed to parse hash".into());
        };
        // after we took the hash, we now have
        // something like [parent, parent, ...]
        // if there is only one parent, it will be of form
        // [parent], so we check if this commit is a merge
        // or not
        let is_merge = match line_split.next() {
            None => false,
            Some(s) => !s.contains(']')
        };
        // if we did find a merge, that means we have to
        // advance our line split until we have reached
        // the end of the [parent, parent, ...] list
        if is_merge { loop {
            match line_split.next() {
                None => (),
                Some(s) => if s.contains(']') {
                    break;
                }
            }
        }}

        let summary = line_split.collect::<Vec<&str>>().join(" ");
        commits.push(Commit {
            summary: summary,
            id: Oid { hash },
            is_merge,
        });
    }

    Ok(commits)
}

pub fn stash(pop: bool) -> io::Result<()> {
    let mut args = vec!["git", "stash"];
    if pop {
        args.push("pop");
    }
    match exec_helpers::execute(&args) {
        Ok(o) => match o.status {
            0 => Ok(()),
            _ => Err(ioerr!("{}", o.stderr)),
        }
        Err(e) => Err(e),
    }
}

pub fn has_modified_files() -> io::Result<bool> {
    let args = ["git", "ls-files", "--modified"];
    match exec_helpers::execute(&args) {
        Ok(o) => match o.status {
            0 => {
                // if stdout is empty, then there are no
                // modified files
                Ok(! o.stdout.trim_end().trim_start().is_empty())
            },
            _ => Err(ioerr!("{}", o.stderr)),
        }
        Err(e) => Err(e),
    }
}

pub fn has_staged_files() -> io::Result<bool> {
    let args = ["git", "diff", "--name-only", "--cached"];
    match exec_helpers::execute(&args) {
        Ok(o) => match o.status {
            0 => {
                // if stdout is empty, then there are no
                // staged files
                Ok(! o.stdout.trim_end().trim_start().is_empty())
            },
            _ => Err(ioerr!("{}", o.stderr)),
        }
        Err(e) => Err(e),
    }
}

pub fn get_number_of_commits_in_ref(refname: &str) -> Result<usize, String> {
    let exec_args = [
        "git", "log", refname, "--format=%H",
    ];
    let mut child = exec_helpers::spawn_with_env_ex(
        &exec_args,
        &[], &[],
        None, None, Some(Stdio::piped()),
    ).map_err(|e| e.to_string())?;

    let stdout = child.stdout.as_mut()
        .ok_or(format!("Failed to get child stdout for reading number of commits of {}", refname))?;
    let stdout_read = BufReader::new(stdout);

    let mut num_lines = 0;
    for line in stdout_read.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if ! line.is_empty() {
            num_lines += 1;
        }
    }
    child.wait().map_err(|e| e.to_string())?;

    Ok(num_lines)
}

pub fn get_repo_root() -> Result<String, String> {
    let exec_args = [
        "git", "rev-parse", "--show-toplevel",
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                // dont want trailing new line
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn fetch_branch(remote: &str, branch: &str) -> Result<(), String> {
    let args = [
        "git", "fetch",
        remote, branch,
        "--no-tags",
    ];

    let err_msg = match exec_helpers::execute(&args) {
        Err(e) => Some(format!("{}", e)),
        Ok(o) => match o.status {
            0 => None,
            _ => Some(o.stderr),
        },
    };
    if let Some(err) = err_msg {
        return Err(err);
    }
    Ok(())
}

pub fn get_all_files_in_repo() -> Result<String, String> {
    let exec_args = [
        "git", "ls-tree", "-r", "HEAD", "--name-only", "--full-tree"
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string())
    }
}

pub fn reset_stage() -> Result<String, String> {
    // git reset --hard
    let exec_args = [
        "git", "reset", "--hard"
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string())
    }
}

/// basically does:
/// git rebase -i --onto onto from~<from_n> from
/// and feeds in the interactive text to the standard input.
/// git rebase interactive will read from the standard input
/// instead of asking user's input. This lets you do
/// git rebase --interactive programatically.
/// the interactive_text should be a string with newlines
/// where each line contains one of the possible commands
/// for an interactive rebase, eg:
/// ```
/// let interactive_text = "pick a022bf message\nfixup bdb0452 other message";
/// ```
pub fn rebase_interactively_with_commits(
    onto: &str,
    from: &str,
    from_n: usize,
    interactive_text: &str,
) -> Result<(), String> {
    let from_n_str = format!("{}~{}", from, from_n);
    let args = [
        "git", "rebase", "-i",
        "--onto", onto,
        &from_n_str, from,
    ];
    let rebase_data_str = format!("echo \"{}\" >", interactive_text);
    // eprintln!("{}", rebase_data_str);
    // eprintln!("{:?}", args);

    let err_msg = match exec_helpers::execute_with_env(
        &args,
        &["GIT_SEQUENCE_EDITOR"],
        &[rebase_data_str.as_str()],
    ) {
        Err(e) => Some(format!("{}", e)),
        Ok(o) => {
            match o.status {
                0 => None,
                _ => Some(o.stderr.lines().next().unwrap().to_string()),
            }
        },
    };
    if let Some(e) = err_msg {
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn oid_short_and_long_works() {
        let oid_str = "692ec5536e98fecfb3dfbce61a5d89af5f2eee34";
        let oid = Oid {
            hash: oid_str.into(),
        };
        let oid_short = oid.short();
        assert_eq!(oid_short, "692ec55");
        let oid_long = oid.long();
        assert_eq!(oid_long, oid_str);
    }

    // just see if it panics or not :shrug:
    #[test]
    #[cfg_attr(not(feature = "gittests"), ignore)]
    fn get_all_commits_from_ref_works() {
        let data = get_all_commits_from_ref("HEAD", None);
        assert!(data.is_ok());
        let data = data.unwrap();
        // this only passes if the test is running from
        // a git repository with more than 1 commit...
        assert!(data.len() > 1);
    }
}
