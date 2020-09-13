use clap::ArgMatches;

use super::git_helpers;
use super::exec_helpers;
use super::split::Runner;
use super::split;
use super::commands::LOCAL_ARG;
use super::commands::REMOTE_ARG;
use super::commands::REMOTE_BRANCH_ARG;
use super::commands::LOCAL_BRANCH_ARG;

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



        self
    }
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
