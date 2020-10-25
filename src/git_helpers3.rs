/// For the v3 version I rewrote the git_helpers module to interface
/// with git via the CLI instead of libgit2

use super::exec_helpers;

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
#[derive(Debug)]
pub struct Commit {
    pub id: Oid,
    pub summary: String,
    pub is_merge: bool,
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
    refname: &str
) -> Result<Vec<Commit>, String> {
    // TODO: in the future might want more info than
    // just the hash and summary
    let exec_args = [
        "git", "log", refname, "--format=%H [%p] %s",
    ];
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
    fn get_all_commits_from_ref_works() {
        let data = get_all_commits_from_ref("HEAD");
        assert!(data.is_ok());
        let data = data.unwrap();
        // this only passes if the test is running from
        // a git repository with more than 1 commit...
        assert!(data.len() > 1);
    }
}
