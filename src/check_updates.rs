use clap::ArgMatches;

use super::git_helpers;
use super::exec_helpers;
use super::split::Runner;
use super::split;
use super::commands::LOCAL_ARG;
use super::commands::REMOTE_ARG;
use super::commands::REMOTE_BRANCH_ARG;
use super::commands::LOCAL_BRANCH_ARG;
use std::path::PathBuf;

pub trait CheckUpdates {
    fn check_updates(self) -> Self;
}

impl<'a> CheckUpdates for Runner<'a> {
    // check if current branch needs to get updates from upstream
    fn check_updates(self) -> Self {
        let mut is_remote = true;
        if self.matches.is_present(LOCAL_ARG[0]) {
            is_remote = false;
        }

        let (current, upstream, upstream_is_remote) = match is_remote {
            true => (get_local_branch(&self), get_remote_branch(&self), true),
            false => (get_remote_branch(&self), get_local_branch(&self), false),
        };
        // nice variable name... easier to read imo
        let current_is_remote = ! upstream_is_remote;

        // whichever is the remote one will be in the format of <uri>?<ref>
        // so we need to know which to be able to split by :

        println!("Checking if {} should get updates from {}", current, upstream);

        // probably want to have two modes eventually:
        // default is to fetch entire remote branch and then run the git diff-tree, and rev-list
        // to determine if theres updates
        // but optionally it would be nice to do an iterative fetch where it just fetches
        // one commit at a time via --deepen=1 (initially it needs to be --depth=1)
        // and then checks the diff-tree on that commit.

        let (remote, branch) = match upstream_is_remote {
            true => get_branch_and_remote_from_str(upstream.as_str()),
            false => get_branch_and_remote_from_str(current.as_str()),
        };

        println!("REMOTE AND BRANCH: {}, {}", remote, branch);
        fetch_branch(remote, branch);

        // TODO
        // match clean_fetch(&self.repo_root_dir) {
        //     Ok(tf) => {
        //         println!("Succesfully deleted FETCH_HEAD");
        //         println!("git prune successful? {}", tf);
        //     },
        //     Err(e) => panic!("Failed to delete FETCH_HEAD:\n{}", e),
        // };

        self
    }
}

fn get_branch_and_remote_from_str(branch_and_remote: &str) -> (&str, &str) {
    let len = branch_and_remote.len();
    let mut last_question_index = len;
    for c in branch_and_remote.chars().rev() {
        if c == '?' {
            break;
        }
        last_question_index -= 1;
    }
    let remote = branch_and_remote.get(0..last_question_index - 1);
    let branch = branch_and_remote.get(last_question_index..len);
    (remote.unwrap(), branch.unwrap())
}

pub fn fetch_branch(remote: &str, branch: &str) {
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
        panic!("Error fetching {} {}\n{}", remote, branch, err);
    }
}

// delete FETCH_HEAD and gc
pub fn clean_fetch(path_to_repo_root: &PathBuf) -> std::io::Result<bool> {
    let mut fetch_head = PathBuf::from(path_to_repo_root);
    fetch_head.push(".git");
    fetch_head.push("FETCH_HEAD");

    if fetch_head.exists() {
        // unwrap and exit... no point in trying
        // to prune if this fails right?
        // git prune will not prune otherwise right?
        std::fs::remove_file(fetch_head)?;
        return Ok(exec_helpers::executed_successfully(&["git", "prune"]));
    }

    Ok(true)
}

fn get_local_branch(runner: &Runner) -> String {
    match runner.matches.value_of(LOCAL_BRANCH_ARG) {
        Some(s) => format!("{}", s),
        None => "HEAD".to_string(),
    }
}

fn get_remote_branch(runner: &Runner) -> String {
    let remote_repo = match runner.repo_file.remote_repo {
        Some(ref s) => s,
        None => panic!("repo file missing remote_repo"),
    };
    // check if user provided a --remote <branch>
    let mut remote_branch = match runner.matches.value_of(REMOTE_BRANCH_ARG[0]) {
        None => "",
        Some(s) => s,
    };
    if remote_branch == "" {
        remote_branch = match runner.repo_file.remote_branch {
            Some(ref s) => s,
            None => "HEAD",
        };
    }

    // format it with a question mark because:
    //    1. we need a way to parse out the branch name
    //    2. a ? is not valid for git branches, so wont conflict
    format!("{}?{}", remote_repo,remote_branch)
}

pub fn run_check_updates(matches: &ArgMatches) {
    let runner = Runner::new(matches);
    runner.save_current_dir()
        .get_repository_from_current_dir()
        .get_repo_file()
        .check_updates();
}
