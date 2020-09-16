use clap::ArgMatches;

use super::git_helpers;
use super::git_helpers::Short;
use super::exec_helpers;
use super::split::Runner;
use super::repo_file::RepoFile;
use super::split;
use super::commands::LOCAL_ARG;
use super::commands::REMOTE_ARG;
use super::commands::REMOTE_BRANCH_ARG;
use super::commands::LOCAL_BRANCH_ARG;
use super::topbase::get_all_blobs_in_branch;
use super::topbase::get_all_blobs_from_commit_with_callback;
use super::topbase::BlobCheckValue;
use super::topbase::BlobCheck;
use super::topbase::blob_check_callback_default;
use std::{collections::HashSet, path::PathBuf};

pub trait CheckUpdates {
    fn check_updates(
        self,
        upstream_branch: &str,
        current_branch: &str,
        current_is_remote: bool,
        should_clean_fetch_head: bool,
        should_summarize: bool,
    ) -> Self;
}

impl<'a> CheckUpdates for Runner<'a> {
    // check if upstream branch needs to get updates from current
    fn check_updates(
        self,
        upstream_branch: &str,
        current_branch: &str,
        current_is_remote: bool,
        should_clean_fetch_head: bool,
        should_summarize: bool,
    ) -> Self {
        let repo = if let Some(ref r) = self.repo {
            r
        } else {
            panic!("Failed to get repo");
        };
        // TODO: probably need to add blob_applies_to_repo_file here?
        // I think in most cases this isnt necessary, but I should
        // try to think of what edge cases this would be needed
        let all_upstream_blobs = get_all_blobs_in_branch(upstream_branch);
        let all_commits_of_current = match git_helpers::get_all_commits_from_ref(repo, current_branch) {
            Ok(v) => v,
            Err(e) => panic!("Failed to get all commits! {}", e),
        };
        // println!("GOT ALL UPSTREAM BLOBS: {}", all_upstream_blobs.len());
        // println!("GOT ALL CURRENT COMMITS: {}", all_commits_of_current.len());

        let mut commits_to_take = vec![];
        let mut commit_summaries = vec![];
        let mut summarize_cb = |c: &git2::Commit| {
            if should_summarize {
                commits_to_take.push(c.id());
                commit_summaries.push(c.summary().unwrap().to_string());
            }
        };
        let should_take_blob_cb = |c: &BlobCheck| {
            if ! blob_path_applies_to_repo_file(&c.path, &self.repo_file, current_is_remote) {
                let blob_check_none: Option<BlobCheckValue> = None;
                return blob_check_none;
            }
            blob_check_callback_default(c)
        };

        // TODO: maybe have different algorithms for checking if theres updates?
        topbase_check_alg_with_callback(
            all_commits_of_current,
            all_upstream_blobs,
            &mut summarize_cb,
            Some(&should_take_blob_cb),
            true,
        );

        if should_summarize {
            summarize_updates(commits_to_take, commit_summaries);
        }

        if should_clean_fetch_head {
            // TODO
            // match clean_fetch(&self.repo_root_dir) {
            //     Ok(tf) => {
            //         println!("Succesfully deleted FETCH_HEAD");
            //         println!("git prune successful? {}", tf);
            //     },
            //     Err(e) => panic!("Failed to delete FETCH_HEAD:\n{}", e),
            // };
        }

        self
    }
}

pub fn summarize_updates(
    commits_to_take: Vec<git2::Oid>,
    commit_summaries: Vec<String>,
) {
    match commits_to_take.len() {
        0 => println!("You are up to date. Latest commit in current exists in upstream"),
        _ => {
            println!("upstream can take {} commit(s) from current:", commits_to_take.len());
            for i in 0..commits_to_take.len() {
                let id = commits_to_take[i];
                let summary = &commit_summaries[i];
                println!("{} {}", id.short(), summary);
            }
        }
    }
}

// evaluate the include/exclude rules of the repo file
// to see if the blob path is relevant to these rules
pub fn blob_path_applies_to_repo_file(
    blob_path: &String,
    repo_file: &RepoFile,
    is_remote: bool,
) -> bool {
    let mut blob_matches_include = false;
    let empty_vec = vec![];
    let include_vec = match &repo_file.include {
        Some(v) => v,
        None => &empty_vec,
    };
    let include_as_vec = match &repo_file.include_as {
        Some(v) => v,
        None => &empty_vec,
    };
    let exclude_vec = match &repo_file.exclude {
        Some(v) => v,
        None => &empty_vec,
    };

    let skip_by = if is_remote { 1 } else { 0 };
    let mut paths_to_include = include_vec.clone();
    for p in include_as_vec.iter().skip(skip_by).step_by(2) {
        paths_to_include.push(p.clone());
    }
    // paths_to_include
    // try to see if it matches any of the include/include_as
    for i in paths_to_include.iter() {
        // remember a single empty space means take anything here
        if i == " " || blob_path.starts_with(i) {
            blob_matches_include = true;
            break;
        }
    }

    // if it matches include, make sure it doesnt match the exclude
    if blob_matches_include {
        for e in exclude_vec {
            if blob_path.starts_with(e) {
                return false;
            }
        }
        return true;
    }

    false
}

// actually its two callbacks... one to build the commit summary,
// the other to decide whether or not to take a blob when
// getting all blobs from commit
pub fn topbase_check_alg_with_callback<F>(
    all_commits_of_current: Vec<git2::Commit>,
    all_upstream_blobs: HashSet<String>,
    cb: &mut F,
    should_take_blob: Option<&dyn Fn(&BlobCheck) -> Option<BlobCheckValue>>,
    // this value is here because if we have filtered out all the blobs
    // in the above should_take_blob callback, then we want to skip this commit
    // and not consider it...
    should_skip_if_no_blobs: bool,
)
    where F: FnMut(&git2::Commit)
{
    // for every commit in the current branch
    // check if every single blob of every commit exists in the upstream branch.
    // as soon as we a commit of this current branch that has all of its blobs
    // exists in upstream, then we break
    for c in all_commits_of_current {
        // I think we want to skip merge commits, because thats what git rebase
        // interactive does by default. also, is it safe to assume
        // any commit with > 1 parent is a merge commit?
        if c.parent_count() > 1 {
            continue;
        }

        let mut current_commit_blobs = HashSet::new();
        get_all_blobs_from_commit_with_callback(
            &c.id().to_string()[..],
            &mut current_commit_blobs,
            should_take_blob,
        );
        let mut all_blobs_exist = true;
        let num_blobs_in_current_commit = current_commit_blobs.len();
        for b in current_commit_blobs {
            if ! all_upstream_blobs.contains(&b) {
                all_blobs_exist = false;
                break;
            }
        }
        // println!("all blobs exist in this comit? {}", all_blobs_exist);
        // println!("num blobs in this commit? {}", num_blobs_in_current_commit);

        if num_blobs_in_current_commit == 0 && should_skip_if_no_blobs {
            continue;
        }

        if all_blobs_exist {
            break;
        }
        cb(&c);
    }
}


pub fn topbase_check_alg<F>(
    all_commits_of_current: Vec<git2::Commit>,
    all_upstream_blobs: HashSet<String>,
    cb: &mut F
)
    where F: FnMut(&git2::Commit),
{
    topbase_check_alg_with_callback(
        all_commits_of_current,
        all_upstream_blobs,
        cb,
        None,
        false,
    );
}

// the above check_updated method will do the checking, and is useful
// for other commands that already have the branch names, and data fetched
// this method will get the information it needs specifically for the check-updates
// command, and fetch it appropriately. it will return the name of the upstream branch
// and the name of the current branch to pass on to the actual check_updates method above
fn setup_check_updates(runner: &Runner) -> (String, String, bool) {
    // 'current' is the branch that potentially
    // has the most recent updates
    let mut current_is_remote = true;
    if runner.matches.is_present(LOCAL_ARG[0]) {
        current_is_remote = false;
    }
    // nice variable name... easier to read imo
    let upstream_is_remote = ! current_is_remote;

    let (current, upstream) = match current_is_remote {
        true => (get_remote_branch(runner), get_local_branch(runner)),
        false => (get_local_branch(runner), get_remote_branch(runner)),
    };

    // whichever is the remote one will be in the format of <uri>?<ref>
    // so we need to know which to be able to split by :
    // checking if upstream should get updates from current
    println!("Current: {}", get_formatted_remote_or_branch_str(&current, current_is_remote));
    println!("Upstream: {}", get_formatted_remote_or_branch_str(&upstream, upstream_is_remote));

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

    // println!("REMOTE AND BRANCH: {}, {}", remote, branch);
    fetch_branch(remote, branch);

    let upstream_branch = match upstream_is_remote {
        true => "FETCH_HEAD".to_string(),
        false => upstream,
    };
    let current_branch = match current_is_remote {
        true => "FETCH_HEAD".to_string(),
        false => current,
    };

    (upstream_branch, current_branch, current_is_remote)
}

fn get_formatted_remote_or_branch_str(branch_and_remote: &str, is_remote: bool) -> String {
    match is_remote {
        false => branch_and_remote.clone().to_string(),
        true => {
            let (remote, branch) = get_branch_and_remote_from_str(branch_and_remote);
            if branch == "HEAD" {
                remote.to_string()
            } else {
                [remote, branch].join(" ")
            }
        },
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
    let runner = runner.save_current_dir()
        .get_repository_from_current_dir()
        .get_repo_file();

    let (upstream, current, current_is_remote) = setup_check_updates(&runner);

    // have to call it with an empty callback...
    // idk how to make it an option, I get weird dyn errors
    runner.check_updates(&upstream[..], &current[..], current_is_remote, true, true);
}
