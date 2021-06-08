// use super::interact;
use super::cli::MgtCommandSync;
use super::core;
use super::die;
use super::git_helpers3;
use super::interact;
use super::repo_file;
use std::{io, path::PathBuf};
use crate::{ioerr, topbase, check::blob_path_applies_to_repo_file};
use git_helpers3::{RawBlobSummary, CommitWithBlobs};
use topbase::SuccessfulTopbaseResult;

/// What kind of sync are we doing? There are 5 possible
/// sync types I can think of:
/// LocalAhead, and RemoteAhead mean one branch is ahead of the other
/// and there is no divergence. Diverged means they are both ahead of
/// some common fork point, UpToDate means both branches top-most commit
/// is a common fork point according to a topbase alg.
/// Disjoint means neither branch has any common fork point between them, so
/// probably cannot sync that easily?
pub enum SyncType {
    LocalAhead, // local is ahead of remote's most recent commit
    RemoteAhead, // remote is ahead of local's most recent commit
    Diverged, // theres differences in local and remote ahead of a common fork point
    UpToDate, // fork point is top-most commit of both
    Disjoint, // failed to find a fork point
}

pub fn get_all_repo_files_ex(list: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out_vec = vec![];
    for path in list {
        if path.is_dir() {
            let all_paths_in_dir = match core::get_all_repo_files(path, true, false) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Skipping {:?} because failed to read repo files:\n{}\n", path, e);
                    continue;
                }
            };

            for p in all_paths_in_dir {
                out_vec.push(PathBuf::from(p));
            }
        } else {
            out_vec.push(path.clone());
        }
    }
    out_vec
}

// TODO: do these :)
pub fn try_sync_in(

) -> io::Result<()> {
    Ok(())
}

pub fn try_sync_out(

) -> io::Result<()> {
    Ok(())
}

// TODO: does this need to be a seperate function?
// wouldnt a sync in do a topbase anyway?
// err, i guess the result would be in
// sync_in_then_out that we rebase
// OUR LOCAL CHANGES on top of whatever
// remote has, and then we push our newly
// rebased local branch out, whereas
// try_sync_in would only take
// the latest remote commits
// and put them ON TOP OF whatever we have locally
pub fn try_sync_in_then_out(

) -> io::Result<()> {
    Ok(())
}

pub fn handle_sync2(
    cmd: &MgtCommandSync,
    remote_url: &str,
    repo_file_path: &PathBuf,
    sync_type: SyncType,
    topbase_success: SuccessfulTopbaseResult<CommitWithBlobs>,
) -> io::Result<()> {
    let (left_ahead, right_ahead) = match sync_type {
        SyncType::LocalAhead |
        SyncType::RemoteAhead |
        SyncType::Diverged => {
            // TODO: technically here if we have a local ahead sync type,
            // we are still looking at the right commits which could contain
            // merge commits for example, and to the user this would look like
            // a divergence event when it doesnt have to be...
            // at what point should merge commits should be filtered from user?
            (&topbase_success.top_commits, &topbase_success.top_right_commits)
        }
        SyncType::UpToDate => {
            println!("Up to date. Nothing to do.");
            return Ok(());
        }
        // this is handled in handle_sync
        SyncType::Disjoint => return Ok(()),
    };

    let mut choices = vec![];
    choices.push("exit");
    choices.push("skip");
    let can_push = ! left_ahead.is_empty();
    let can_pull = ! right_ahead.is_empty();
    if can_push {
        choices.push("push");
        println!("\nYou can push:");
        for commit in left_ahead {
            println!("  {} {}", commit.commit.id.short(), commit.commit.summary);
        }
    }
    if can_pull {
        choices.push("pull");
        println!("\nYou can pull:");
        for commit in right_ahead {
            println!("  {} {}", commit.commit.id.short(), commit.commit.summary);
        }
    }
    if can_pull && can_push {
        choices.push("pull --rebase, then push");
        // TODO: other cool options.
        // eg: be able to interactively choose how the pull gets rebased
        // into your local changes by dragging and dropping the
        // commits in your local changes in the order you want...
        // advanced feature, might want to look into ncurses type lib
        // because thats probably too complex to do myself
    }

    // the nicest order is actually the reverse because
    // we want exit and skip to be at the bottom:
    choices.reverse();

    println!();
    let i_choices: interact::InteractChoices = (&choices[..]).into();
    let selection = interact::interact_number(i_choices)?;
    let selection_index = selection - 1;
    let selection = choices[selection_index];

    match selection {
        "skip" => return Ok(()),
        "exit" => std::process::exit(0),
        "pull" => try_sync_in(),
        "push" => try_sync_out(),

        // this is pull --rebase then push:
        _ => try_sync_in_then_out(),
    }
}

pub fn handle_sync(
    cmd: &MgtCommandSync,
    remote_url: &str,
    repo_file_path: &PathBuf,
    sync_type: SyncType,
    topbase_opt: Option<SuccessfulTopbaseResult<CommitWithBlobs>>,
) -> io::Result<()> {
    match topbase_opt {
        Some(s) => handle_sync2(cmd, remote_url, repo_file_path, sync_type, s),
        None => {
            // TODO: come up with something better than just saying this
            println!("Branches are disjoint. cannot sync");
            Ok(())
        }
    }
}

pub fn sync_repo_file(
    starting_branch_name: &str,
    repo_file_path: &PathBuf,
    cmd: &MgtCommandSync,
) -> io::Result<()> {
    let repo_file = repo_file::parse_repo_file_from_toml_path_res(
        repo_file_path)?;
    let default_branch = "HEAD".to_string();
    // TODO: do repo files support specifying a branch name for the remote?
    // is defaulting to HEAD ok?
    let repo_url = repo_file.remote_repo.as_ref()
        .ok_or(ioerr!("Failed to find a remote repo in the repo file: {:?}", repo_file_path))?;
    let repo_branch = repo_file.remote_branch.as_ref().unwrap_or(&default_branch);

    let divider = "=".repeat(15);
    println!("\n{} Fetching {}:{} {}", divider, repo_url, repo_branch, divider);
    git_helpers3::fetch_branch(repo_url, repo_branch).map_err(|e| ioerr!("{}", e))?;

    // TODO: support sync from a different branch other than the one
    // we are currently on?
    let local_branch = "HEAD";
    let remote_branch = "FETCH_HEAD";

    // this is important because we are not filtering any of the paths, so
    // with BlobHashingMode::Full, we would fail to find the correct fork point
    // because the paths are most likely different
    let hashing_mode = topbase::BlobHashingMode::WithoutPath;
    // TODO: make cli option?
    let traverse_at_a_time = 500;
    // we don't know which one is ahead, so we want to rewind the B branch
    // and see where the differences are from the most recent fork point
    let should_rewind = true;
    let should_use_blob_cb = |c: &mut RawBlobSummary, b: &str| {
        let this_is_a_remote_blob = b == remote_branch;
        blob_path_applies_to_repo_file(&c.path_str, &repo_file, this_is_a_remote_blob)
    };
    let topbase_ok = topbase::find_a_b_difference2::<CommitWithBlobs, _>(
        local_branch, remote_branch, Some(traverse_at_a_time),
        hashing_mode, should_rewind, Some(should_use_blob_cb))?;
    let sync_type = match topbase_ok {
        None => SyncType::Disjoint,
        Some(ref o) => {
            let local_empty = o.top_commits.is_empty();
            let remote_empty = o.top_right_commits.is_empty();
            // TODO: handle merge commit filtering here.
            // if user provides --allow-merges option
            // then we consider a merge commit a potential divergence,
            // otherwise, we have to check the bottom case here
            // and if one of the branches only contains merge commits,
            // we pretend they dont exist.
            match (local_empty, remote_empty) {
                (true, true) => SyncType::UpToDate,
                (true, false) => SyncType::RemoteAhead,
                (false, true) => SyncType::LocalAhead,
                (false, false) => SyncType::Diverged,
            }
        }
    };
    handle_sync(cmd, repo_url, repo_file_path, sync_type, topbase_ok)
}

pub fn canonicalize_all_repo_file_paths(paths: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out_paths = vec![];
    for p in paths {
        match p.canonicalize() {
            Ok(canon) => {
                out_paths.push(canon);
            }
            Err(e) => {
                eprintln!("Not including {:?} because failed to canonicalize path\n{}", p, e);
            }
        };
    }
    out_paths
}

// TODO: need a way to cleanup if any error...
// couple things here:
// 1. cleanup should be on an individual sync item level
// ie: if syncing branch A fails, then that sync operation
// should fix itself, so if the user wants to continue to syncing
// branch B, the repo is in a valid state to continue syncing.
// 2. operations while syncing should be reversible.
// ie: we need to know before doing something what the appropriate
// action would be if we fail. consider for example
// doing a pull --rebase... obviously there could be conflicts.
// and if so, we want to be able to get the user's local branch
// back to the exact state it started in...

// TODO: rewrite sync repo file to not actually topbase/filter
// we can do what the check command does, and just get the FETCH_HEAD
// for each repo file, and then perform a topbase search without path
// hash mode, and based on that output we will find out
// what we want to know for sync, ie:
// are we ahead of remote, are we behind, both?, or neither ahead nor behind (ie: no fork point)?

pub fn run_sync(cmd: &mut MgtCommandSync) {
    // before we go to the repo root, we want to canonicalize
    // all of the paths the user provided, otherwise they wont work anymore
    // from a new directory
    cmd.repo_files = canonicalize_all_repo_file_paths(&cmd.repo_files);
    core::verify_dependencies();
    core::go_to_repo_root();

    // core::safe_to_proceed();
    // TODO:
    // ideally itd be nice if user just wants to run through and look
    // at the sync options available (which is totally possible because
    // we are just fetching, and dont need to rewrite history here
    // unless the user requests an action!), and not be asked
    // to stash recent changes... actually maybe
    // we can ask them interactively:
    // 1. stash and continue
    // 2. preview sync without being able to pull/push
    // 3. exit and manually stash/commit changes

    let starting_branch_name = core::get_current_ref().unwrap_or_else(|| {
        die!("Failed to get current branch name. Cannot continue")
    });
    let mut all_repo_files = get_all_repo_files_ex(&cmd.repo_files);
    println!("Found {:#?} repo files to sync", all_repo_files);
    println!("Found {} repo files to sync", all_repo_files.len());

    for (_index, repo_file) in all_repo_files.drain(..).enumerate() {
        let potential_err = format!("Error trying to sync {:?} :", repo_file);
        if let Err(e) = sync_repo_file(&starting_branch_name, &repo_file, cmd) {
            eprintln!("{}\n{}", potential_err, e);
            if cmd.fail_fast {
                // TODO: do we need a single global cleanup?
                // or only cleanup on a per item basis?
                // leaning towards just cleaning up each individual sync...
                // attempt_cleanup_or_die(branches_to_delete, starting_branch_name);
                std::process::exit(1);
            }
        }
    }
}
