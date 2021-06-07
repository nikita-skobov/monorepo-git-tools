use std::path::PathBuf;

use super::git_helpers3;
use super::git_helpers3::Oid;
use super::exec_helpers;
use super::repo_file::RepoFile;
use super::die;
use super::topbase;
use super::repo_file;
use super::cli::MgtCommandCheck;
use super::core::get_all_repo_files;
use git_helpers3::CommitWithBlobs;

pub struct Checker<'a> {
    upstream_branch: String,
    current_branch: String,
    current_is_remote: bool,
    repo_file: &'a RepoFile,
}

impl<'a> Checker<'a> {
    /// just an alias to call create_checker via Checker::create
    /// instead of calling the function itself :shruf:
    pub fn create<S: ToString>(
        current_is_remote: bool,
        local_branch: Option<S>,
        remote_branch: Option<S>,
        repo_file: &'a RepoFile,
    ) -> Checker {
        create_checker(current_is_remote, local_branch, remote_branch, repo_file)
    }

    pub fn check_for_updates(
        &self,
        repo_file_path: Option<&str>,
        should_clean_fetch_head: bool,
        should_summarize: bool,
    ) {
        let (commits_to_take, commit_summaries) = check_for_updates(
            self.repo_file,
            &self.upstream_branch,
            &self.current_branch,
            self.current_is_remote,
            should_summarize
        );

        if should_summarize {
            let command_to_take = match repo_file_path {
                None => None,
                Some(file_path) => {
                    let split_mode = if self.current_is_remote {
                        "split-in"
                    } else {
                        "split-out"
                    };
                    // TODO: calculate if it can be topbased/rebased/whatever...
                    // here we just assume that it can be topbased...
                    let command_string = "To perform this update you can run: ";
                    let command_string = format!("\n{}\nmgt {} {} --topbase", command_string, split_mode, file_path);
                    Some(command_string)
                }
            };
            summarize_updates(command_to_take, commits_to_take, commit_summaries);
        }

        if should_clean_fetch_head {
            // TODO: clean fetch head...
            // hard to do because of gits auto gc?
        }
    }
}

pub fn summarize_updates(
    command_string: Option<String>,
    commits_to_take: Vec<Oid>,
    commit_summaries: Vec<String>,
) {
    match commits_to_take.len() {
        0 => println!("You are up to date. Latest commit in current exists in upstream"),
        _ => {
            println!("upstream can take {} commit(s) from current:", commits_to_take.len());
            for i in 0..commits_to_take.len() {
                let id = &commits_to_take[i];
                let summary = &commit_summaries[i];
                println!("{} {}", id.short(), summary);
            }
            if let Some(command_string) = command_string {
                println!("{}", command_string);
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
    if let Err(e) = git_helpers3::fetch_branch(remote, branch) {
        die!("Error fetching {} {}\n{}", remote, branch, e);
    }
}

// delete FETCH_HEAD and gc
pub fn _clean_fetch(path_to_repo_root: &PathBuf) -> std::io::Result<bool> {
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

pub fn run_check(cmd: &mut MgtCommandCheck) {
    // remote is true by default, unless --local
    // was specified
    if !cmd.local {
        cmd.remote = true;
    }

    let repo_file_path = if cmd.repo_file.len() < 1 {
        die!("Must provide repo file path");
    } else {
        cmd.repo_file[0].clone()
    };

    let repo_file_pathbuf: PathBuf = repo_file_path.clone().into();
    let files_to_check = if repo_file_pathbuf.is_file() {
        vec![repo_file_path.to_string()]
    } else {
        // iterate over that folder and find all repo files
        let repo_files = get_all_repo_files(
            &repo_file_path,
            cmd.recursive,
            cmd.all,
        );
        match repo_files {
            Ok(files) => files,
            Err(e) => die!("Failed to read repo file directory: {}", e),
        }
    };

    for file in files_to_check {
        println!("---\nChecking {}", file);
        let repo_file = repo_file::parse_repo_file_from_toml_path(&file);
        let current_is_remote = cmd.remote;
        let checker = Checker::create(
            current_is_remote,
            cmd.local_branch.clone(),
            cmd.remote_branch.clone(),
            &repo_file
        );
        checker.check_for_updates(
            Some(&file),
            true,
            true
        );
    }
}

/// create the checker struct that is setup and ready
/// to run the check operation
pub fn create_checker<S: ToString>(
    current_is_remote: bool,
    local_branch: Option<S>,
    remote_branch: Option<S>,
    repo_file: &RepoFile,
) -> Checker {
    // 'current' is NOT the branch we are currently on
    // but rather its the branch that potentially
    // has the most recent updates
    let upstream_is_remote = ! current_is_remote;

    let current = get_current_branch_name(
        repo_file,
        current_is_remote,
        &local_branch,
        &remote_branch,
    );
    let upstream = get_upstream_branch_name(
        repo_file,
        current_is_remote,
        &local_branch,
        &remote_branch,
    );

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

    Checker { upstream_branch, current_branch, current_is_remote, repo_file }
}

fn get_current_branch_name<S: ToString>(
    repo_file: &RepoFile,
    current_is_remote: bool,
    local_branch: &Option<S>,
    remote_branch: &Option<S>,
) -> String {
    if current_is_remote {
        get_remote_branch2(repo_file, remote_branch)
    } else {
        match local_branch {
            Some(ref s) => s.to_string(),
            None => "HEAD".to_string(),
        }
    }
}

/// this does the same as get_current_branch_name
/// except it should do the opposite based
/// on the current_is_remote flag
fn get_upstream_branch_name<S: ToString>(
    repo_file: &RepoFile,
    current_is_remote: bool,
    local_branch: &Option<S>,
    remote_branch: &Option<S>,
) -> String {
    get_current_branch_name(repo_file, !current_is_remote, local_branch, remote_branch)
}

fn get_remote_branch2<S: ToString>(
    repo_file: &RepoFile,
    remote_branch: &Option<S>,
) -> String {
    let remote_repo = match repo_file.remote_repo {
        Some(ref s) => s,
        None => die!("repo file missing remote_repo"),
    };
    // check if user provided a --remote <branch>
    let remote_branch = match remote_branch {
        Some(ref s) => s.to_string(),
        None => match repo_file.remote_branch {
            Some(ref s) => s.to_string(),
            None => "HEAD".to_string(),
        }
    };

    // format it with a question mark because:
    //    1. we need a way to parse out the branch name
    //    2. a ? is not valid for git branches, so wont conflict
    format!("{}?{}", remote_repo,remote_branch)
}

/// check if upstream branch needs to get updates from current
fn check_for_updates(
    repo_file: &RepoFile,
    upstream_branch: &str,
    current_branch: &str,
    current_is_remote: bool,
    should_summarize: bool,
) -> (Vec<Oid>, Vec<String>) {
    // we need to enable rewind mode if our current branch
    // is on the right.
    let mut should_rewind = false;
    let (a_branch, b_branch) = if current_is_remote {
        (current_branch, upstream_branch)
    } else {
        should_rewind = true;
        (upstream_branch, current_branch)
    };

    let hashing_mode = topbase::BlobHashingMode::WithoutPath;
    let traverse_at_a_time = 500;
    let mut out_ids = vec![];
    let mut out_str = vec![];
    let successful_topbase = match topbase::find_a_b_difference2::<CommitWithBlobs>(
        a_branch, b_branch, Some(traverse_at_a_time), hashing_mode, should_rewind)
    {
        Ok(s) => if let Some(t) = s { t } else { return (out_ids, out_str) },
        Err(_) => return (out_ids, out_str),
    };

    // if we should rewind, that means we expect the commits that upstream
    // wants are on the 'right' side (ie: current branch is the B branch,
    // and we always care about upstream getting updates from current).
    // so we iterate the top_right_commits instead of the top_commits
    // in that case:
    let iter_commits = if should_rewind {
        successful_topbase.top_right_commits.iter()
    } else {
        successful_topbase.top_commits.iter()
    };

    for out_commit in iter_commits {
        // check all blob paths to make sure they apply
        // to our repo file:
        let mut all_blob_paths_apply = true;
        for blob_info in out_commit.blobs.iter() {
            let blob_path = &blob_info.path_str;
            if ! blob_path_applies_to_repo_file(blob_path, repo_file, current_is_remote) {
                all_blob_paths_apply = false;
                break;
            }
        }
        if ! all_blob_paths_apply { continue; }
        if should_summarize {
            out_ids.push(out_commit.commit.id.clone());
            out_str.push(out_commit.commit.summary.clone());
        }
    }

    (out_ids, out_str)
}
