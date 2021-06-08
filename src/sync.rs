// use super::interact;
use super::cli::MgtCommandSync;
use super::core;
use super::die;
use super::git_helpers3;
use super::interact;
use super::repo_file;
use std::{io, path::PathBuf};
use crate::{ioerr, topbase, check::blob_path_applies_to_repo_file, split_out::generate_gitfilter_filterrules, ioerre, split_in};
use git_helpers3::{RawBlobSummary, CommitWithBlobs, Commit};
use topbase::SuccessfulTopbaseResult;
use repo_file::RepoFile;
use std::{fmt::Display, time::{Duration, SystemTime}, process::Stdio};
use gitfilter::filter::FilterRule;

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

pub fn make_random_branch_name(backup_number: usize) -> String {
    let now = SystemTime::now();
    match now.duration_since(SystemTime::UNIX_EPOCH) {
        Err(_) => format!("mgt-tmp-branch-delete-later-{}", backup_number),
        Ok(n) => {
            format!("mgt-tmp-branch-delete-later-{}", n.as_secs())
        }
    }
}

pub fn try_checkout_back_to_starting_branch<E: Display>(
    starting_branch_name: &str,
    original_error: E,
) -> io::Result<()> {
    // cleanup operation?
    // probably none? most likely this is a branch
    // that already existed, so no need to
    // do anything other than tell user, oops, we failed
    // to make this branch...
    // also should verify we are back on the starting branch:
    let mut err_msg = format!("{}", original_error);
    let current_branch = git_helpers3::get_current_ref();
    let should_try_to_checkout_back = match current_branch {
        Ok(bn) => bn != starting_branch_name,
        Err(e) => {
            err_msg = format!("{}\nALSO: while trying to ensure we are back on {}, failed to get current branch name because:\n{}\nTrying to checkout back to {} anyway", err_msg, starting_branch_name, e, starting_branch_name);
            true
        }
    };
    if should_try_to_checkout_back {
        eprintln!("- Switching back to {}", starting_branch_name);
        if let Err(e) = git_helpers3::checkout_branch(starting_branch_name, false) {
            err_msg = format!("{}\nALSO: failed to checkout back to {} because:\n{}\nThis is probably a bug; please report this.", err_msg, starting_branch_name, e);
        }
    }

    return ioerre!("{}", err_msg);
}

pub fn try_checkout_new_branch(
    branch: &str,
    starting_branch_name: &str
) -> io::Result<()> {
    let make_new = true;
    let branch_made = git_helpers3::checkout_branch(&branch, make_new);
    if let Err(e) = branch_made {
        let err_msg = format!("Failed to create a temporary branch {} because:\n{}\nDoes this branch already exist maybe?", branch, e);
        return try_checkout_back_to_starting_branch(starting_branch_name, err_msg);
    }

    Ok(())
}

pub fn try_making_branch_from(
    branch_name: &str,
    make_from: &str,
    starting_branch_name: &str
) -> io::Result<()> {
    let exec_args = [
        "git", "branch", branch_name, make_from
    ];
    if let Some(err) = exechelper::executed_with_error(&exec_args) {
        let err_msg = format!("Failed to create a temporary branch {} because:\n{}Does this branch already exist maybe?", branch_name, err);
        return try_checkout_back_to_starting_branch(starting_branch_name, err_msg);
    }

    // TODO: like i pointed out in a comment in the try_sync_out
    // function, I think being on the branch thats to be filtered
    // isnt even necessary..
    // so todo is to remove this:
    let make_new = false;
    let branch_made = git_helpers3::checkout_branch(branch_name, make_new);
    if let Err(e) = branch_made {
        let err_msg = format!("Failed to checkout to temporary branch {} because:\n{}", branch_name, e);
        return try_checkout_back_to_starting_branch(starting_branch_name, err_msg);
    }

    Ok(())
}

pub fn try_delete_branch<E: Display>(
    branch: &str,
    original_error: E,
) -> io::Result<()> {
    eprintln!("- Deleting {}", branch);
    if let Err(e) = git_helpers3::delete_branch(branch) {
        return ioerre!("{}\nALSO: Failed to delete branch {} when trying to recover because\n{}", original_error, branch, e);
    }

    Ok(())
}

pub fn try_perform_gitfilter(
    branch: String,
    starting_branch_name: &str,
    filter_rules: Vec<FilterRule>,
) -> io::Result<String> {
    let is_verbose = false;
    let is_dry_run = false;
    let filtered = core::perform_gitfilter_res(
        filter_rules,
        branch.clone(),
        is_dry_run,
        is_verbose,
    );
    if let Err(e) = filtered {
        // cleanup operation?
        // TODO: tricky one. probably need
        // to check the state of this branch. Is it
        // possible there was some conflict and we have a big
        // git stage set up? in that case, wed need to
        // clear the stage index, and then go back to the
        // starting branch... could be several things wrong here.
        let err_msg = format!("Failed to perform gitfilter on branch {} because\n{}", branch, e);
        let err_msg = if let Err(e) = try_checkout_back_to_starting_branch(starting_branch_name, &err_msg) {
            // failed to go back to starting branch
            // TODO: need to do anything else here?
            return Err(e);
        } else {
            // succeeded in going back to our starting branch,
            // now lets try to delete the temporary branch that
            // we wanted to filter:
            if let Err(e) = try_delete_branch(&branch, &err_msg) {
                return Err(e);
            }
            err_msg
        };


        return ioerre!("{}", err_msg);
    }
    Ok(branch)
}

pub fn try_rebase_onto(
    onto_fork_point: &str,
    top_name: &str,
    top_num_commits: usize,
    interactive_rebase_str: &str,
) -> io::Result<()> {
    let rebase_res = git_helpers3::rebase_interactively_with_commits(
        onto_fork_point, top_name, top_num_commits, interactive_rebase_str);

    if let Err(err) = rebase_res {
        return ioerre!("Failed to rebase top {} commits of {} onto {} because\n{}\nLeaving you with a git interactive rebase in progress. Go back with 'git rebase --abort', or otherwise rebase manually and then finish with 'git rebase --continue'", top_num_commits, top_name, onto_fork_point, err);
    }

    Ok(())
}

pub fn try_get_output_branch_name(
    random_branch: &str,
    starting_branch_name: &str,
) -> io::Result<String> {
    let message = "Enter the desired branch name to be created on the remote repo (hit Enter to use an auto-generated branch name)";
    let interact_choice = interact::InteractChoices::choose_word(&message);
    let push_branch_name = interact::interact_word(interact_choice)
        .map_err(|err| {
            // failed to interact, but instead of just exiting here,
            // we still need to cleanup.
            if let Err(e) = try_checkout_back_to_starting_branch(starting_branch_name, &err) {
                return e;
            }
            if let Err(e) = try_delete_branch(&random_branch, &err) {
                return e;
            }
            err
        })?;
    let push_branch_name = push_branch_name.trim_end().trim_start();
    let push_branch_name = if push_branch_name.is_empty() {
        // if its empty, user hit enter, and then we use the default
        // which is the auto generated branch name
        &random_branch
    } else { push_branch_name };

    Ok(push_branch_name.to_string())
}

/// NOTE: obviously pushing to a remote repo requires authentication.
/// I don't want to add auth logic to mgt (at least for now), but I think
/// the following is a good solution:
/// we simply inherit the stdin of the git push command.
/// If the user is using a regular https git url, then chances
/// are that theyll be asked for a user/password, and they just enter
/// it into their terminal and it works! However, for syncing many
/// repos this can be annoying to do each time... mgt WILL NOT try
/// to make that easier on the user by storing/caching their credentials...
/// git already has that capability, so if the user wants push to not
/// ask for their credentials each time, they should use git credential store
/// or git credential cache, or better yet, use ssh-agent with an ssh key
/// that they authorize before running mgt.
pub fn try_push_out(
    remote_url: &str,
    random_branch: &str,
    push_branch: &str,
    starting_branch_name: &str,
) -> io::Result<()> {
    let push_branch_ref = format!("{}:{}", random_branch, push_branch);
    let exec_args = [
        "git", "push", remote_url, &push_branch_ref
    ];

    let child = exechelper::spawn_with_env_ex(
        &exec_args, &[], &[], Some(Stdio::inherit()),
        Some(Stdio::piped()), Some(Stdio::piped())).map_err(|err| {
            // failed to start child, but instead of just exiting here,
            // we still need to cleanup.
            if let Err(e) = try_checkout_back_to_starting_branch(starting_branch_name, &err) {
                return e;
            }
            if let Err(e) = try_delete_branch(&random_branch, &err) {
                return e;
            }
            err
        })?;
    let output = child.wait_with_output().map_err(|err| {
        // failed to run command successfully to the end
        if let Err(e) = try_checkout_back_to_starting_branch(starting_branch_name, &err) {
            return e;
        }
        if let Err(e) = try_delete_branch(&random_branch, &err) {
            return e;
        }
        err
    })?;
    let out_err = if ! output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let err = format!("Failed to run git push command:\n{}", stderr);
        Some(err)
    } else { None };
    if let Some(err) = out_err {
        // failed to run command successfully to the end
        try_checkout_back_to_starting_branch(starting_branch_name, &err)?;
        try_delete_branch(&random_branch, &err)?;
        return ioerre!("{}", err);
    }

    // At this point we have made a successful git push
    Ok(())
}

pub fn get_new_commits_after_filter(
    filtered_branch_name: &str,
    commits_before_filter: &Vec<CommitWithBlobs>,
) -> io::Result<Vec<Commit>> {
    let desired_commits = commits_before_filter.len();
    let commits = git_helpers3::get_all_commits_from_ref(
        filtered_branch_name, Some(desired_commits)).map_err(|e| ioerr!("{}", e))?;

    // TODO: is just using the number of commits
    // that we set from -n <desired_commits> sufficient?
    // what we are doing here is very implicit:
    // we believe that after we filtered the branch, that
    // the top N commits we wanted before are still all there, ie:
    // that they did not get filtered out.
    // this is probably a reasonable assumption, but if theres ever
    // a weird bug of 'why isnt this commit being included?'
    // look here...
    Ok(commits)
}

pub fn try_get_new_commits_after_filter(
    filtered_branch_name: &str,
    commits_before_filter: &Vec<CommitWithBlobs>,
    starting_branch_name: &str,
) -> io::Result<Vec<Commit>> {
    match get_new_commits_after_filter(filtered_branch_name, commits_before_filter) {
        Ok(c) => Ok(c),
        Err(e) => {
            // failed, try to recover:
            try_checkout_back_to_starting_branch(starting_branch_name, &e)?;
            try_delete_branch(&filtered_branch_name, &e)?;
            return ioerre!("{}", e);
        }
    }
}

pub fn get_rebase_interactive_string_and_number(
    commits_to_take: &Vec<Commit>
) -> (usize, String) {
    // ok when we do git rebase -i <from>~N <from>
    // we are not guarnateed that rebase will actually
    // only take N, especially because our branches have no related history...
    // so thats why we pass this interactive rebase string of exactly
    // which commits we want.
    // the problem is that we only know the commit hashes of these before
    // that branch got filtered. so we need to run
    // git log now, and find the new hashes for these commits for us
    // to correctly send the interactive rebase string
    let mut num_commits_to_take = 0;
    let mut rebase_interactive_segments: Vec<String> = commits_to_take.iter().map(|c| {
        if c.is_merge {
            // this skips the merge commit
            "".to_string()
        } else {
            num_commits_to_take += 1;
            format!("pick {} {}\n", c.id.long(), c.summary)
        }
    }).collect();
    rebase_interactive_segments.reverse();
    let rebase_interactive_string = rebase_interactive_segments.join("");
    (num_commits_to_take, rebase_interactive_string)
}

/// returns true if user wants to merge
pub fn try_get_merge_choice(
    branch_name: &str,
    starting_branch_name: &str,
) -> io::Result<bool> {
    let ff_merge_str = format!("Merge {} into {} by fast-forwarding", starting_branch_name, branch_name);
    let rename_option = format!("Leave the {} branch as is, and manually merge after review", branch_name);
    let merge_options = [
        &ff_merge_str,
        &rename_option,
    ];
    let mut merge_choices: interact::InteractChoices = (&merge_options[..]).into();
    let description = "Would you like to merge your original branch into the newly filtered branch?".to_string();
    merge_choices.description = Some(description);
    let finalize_choice = interact::interact_number(merge_choices);
    let finalize_choice = match finalize_choice {
        Ok(c) => c,
        Err(e) => {
            // failed, try to recover:
            try_checkout_back_to_starting_branch(starting_branch_name, &e)?;
            try_delete_branch(branch_name, &e)?;
            return ioerre!("{}", e);
        }
    };
    Ok(finalize_choice == 1)
}

pub fn try_fast_forward_merge(
    branch_name: &str,
    starting_branch_name: &str
) -> io::Result<()> {
    // try to ff merge into the temp branch
    let exec_args = [
        "git", "merge", "--ff-only", branch_name,
    ];
    if let Some(e) = exechelper::executed_with_error(&exec_args) {
        // TODO: can we recover if we failed to ff-merge?
        // this could be a conflict resolution so maybe we can ask user if
        // they want to manually review it, or abort?
        return ioerre!("Failed to merge {} into {} because\n:{}", starting_branch_name, branch_name, e);
    }

    Ok(())
}

// TODO: do these :)
// AKA: pull remote changes into local
pub fn try_sync_in(
    repo_file: &RepoFile,
    starting_branch_name: &str,
    fork_point_local: &str,
    // num_commits_to_pull: usize,
    commits_to_pull: &Vec<CommitWithBlobs>,
) -> io::Result<()> {
    let is_verbose = false;
    let filter_rules = split_in::generate_gitfilter_filterrules(&repo_file, is_verbose);
    let random_number = match repo_file.remote_repo {
        Some(ref s) => s.len(),
        None => 123531421321, // very secure, got it from some .gov website
    };
    println!("- Making temporary branch");
    let random_branch = make_random_branch_name(random_number);
    // TODO: we are assuming here that the remote code was pulled into
    // FETCH_HEAD. will this always be the case?
    try_making_branch_from(&random_branch, "FETCH_HEAD", starting_branch_name)?;

    println!("- Filtering branch according to repo file");
    let random_branch = try_perform_gitfilter(
        random_branch, starting_branch_name, filter_rules)?;

    let new_commits_to_pull = try_get_new_commits_after_filter(&random_branch, &commits_to_pull, starting_branch_name)?;
    let (num_commits_to_pull, rebase_interactive_string) = get_rebase_interactive_string_and_number(
        &new_commits_to_pull);

    println!("- Rebasing onto calculated fork point");
    try_rebase_onto(fork_point_local, &random_branch,
        num_commits_to_pull, &rebase_interactive_string)?;
    println!("- Successfully rebased temporary branch");

    // TODO: what about cli arguments to not ask this:
    // eg: --always-merge or something
    let user_wants_to_merge = try_get_merge_choice(&random_branch, starting_branch_name)?;
    if user_wants_to_merge {
        // to fast forward merge i believe we have to be
        // on the starting branch to do that...
        println!("- Checking out back to {}", starting_branch_name);
        if let Err(e) = git_helpers3::checkout_branch(starting_branch_name, false) {
            return ioerre!("failed to checkout back to {} because:\n{}\nThis is probably a bug; please report this.", starting_branch_name, e);
        }

        println!("- Fast-forward merging {}", starting_branch_name);
        try_fast_forward_merge(&random_branch, starting_branch_name)?;
        // if that succeeded, then we can delete the temporary branch
        println!("- Successfully merged. Deleting temporary branch");
        git_helpers3::delete_branch(&random_branch).map_err(|e| ioerr!("{}", e))?;
        return Ok(());
    }
    // otherwise, if user did not want to merge,
    // we do not delete the branch because obviously the user
    // wants to review it.
    // so I guess we are done here.
    println!("- Leaving you on {} to review and manually merge", random_branch);

    Ok(())
}

/// AKA: push local changes to remote
// split-out BUT DONT USE the topbase module
// since we just did a fetch, and already ran an in-memory
// topbase, we now know the fork point, so we can
// just rebase onto that fork point thats currently
// in our FETCH_HEAD
pub fn try_sync_out(
    repo_file: &RepoFile,
    repo_remote_url: &str,
    starting_branch_name: &str,
    fork_point_remote: &str,
    commits_to_push: &Vec<CommitWithBlobs>,
) -> io::Result<()> {
    let is_verbose = false;
    let filter_rules = generate_gitfilter_filterrules(&repo_file, is_verbose);
    let random_number = match repo_file.remote_repo {
        Some(ref s) => s.len(),
        None => 123531421321, // very secure, got it from some .gov website
    };
    println!("- Making temporary branch");
    let random_branch = make_random_branch_name(random_number);
    // TODO: I think checking out to this new branch isnt even necessary?
    // i think we can filter that branch without being on it, and then
    // also rebase without being on it... if thats true, then
    // the user can always stay on their branch, and we just make a new
    // tmp branch from their current branch
    try_checkout_new_branch(&random_branch, starting_branch_name)?;

    println!("- Filtering branch according to repo file");
    let random_branch = try_perform_gitfilter(
        random_branch, starting_branch_name, filter_rules)?;

    let new_commits_to_push = try_get_new_commits_after_filter(&random_branch, &commits_to_push, starting_branch_name)?;
    let (num_commits_to_push, rebase_interactive_string) = get_rebase_interactive_string_and_number(
        &new_commits_to_push);

    println!("- Rebasing onto calculated fork point");
    try_rebase_onto(fork_point_remote, &random_branch, num_commits_to_push, &rebase_interactive_string)?;

    let push_branch_name = try_get_output_branch_name(&random_branch, starting_branch_name)?;
    println!("- git push {} {}:{}", repo_remote_url, random_branch, push_branch_name);
    try_push_out(repo_remote_url, &random_branch, &push_branch_name, starting_branch_name)?;

    println!("- Successfully git pushed. Changing back to original branch: {}", starting_branch_name);
    if let Err(e) = git_helpers3::checkout_branch(starting_branch_name, false) {
        return ioerre!("failed to checkout back to {} because:\n{}\nThis is probably a bug; please report this.", starting_branch_name, e);
    }
    println!("- Deleting temporary branch");
    if let Err(e) = git_helpers3::delete_branch(&random_branch) {
        return ioerre!("failed to delete branch {} because:\n{}\nThis is probably a bug; please report this.", &random_branch, e);
    }

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
    println!("Not implemented yet, skipping...");
    Ok(())
}

pub fn handle_sync2(
    cmd: &MgtCommandSync,
    remote_url: &str,
    repo_file_path: &PathBuf,
    repo_file: &RepoFile,
    sync_type: SyncType,
    topbase_success: SuccessfulTopbaseResult<CommitWithBlobs>,
    starting_branch_name: &str,
) -> io::Result<()> {
    let (left_ahead, right_ahead) = match sync_type {
        SyncType::LocalAhead |
        SyncType::RemoteAhead |
        SyncType::Diverged => {
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
    let mut can_push = ! left_ahead.is_empty();
    let mut can_pull = ! right_ahead.is_empty();
    if can_push {
        let mut has_non_merge = false;
        let mut out_str = "\nYou can push:".to_string();
        for commit in left_ahead {
            if ! commit.commit.is_merge {
                has_non_merge = true;
                out_str = format!("{}\n  {} {}", out_str, commit.commit.id.short(), commit.commit.summary);
            }
        }
        if has_non_merge {
            choices.push("push");
            println!("{}", out_str);
        } else {
            // if there are ONLY merge commits, then say
            // that we cannot push:
            can_push = false;
        }
    }
    if can_pull {
        let mut has_non_merge = false;
        let mut out_str = "\nYou can pull:".to_string();
        for commit in right_ahead {
            if ! commit.commit.is_merge {
                has_non_merge = true;
                out_str = format!("{}\n  {} {}", out_str, commit.commit.id.short(), commit.commit.summary);
            }
        }
        if has_non_merge {
            choices.push("pull");
            println!("{}", out_str);
        } else {
            // if ONLY merge commits, then say
            // we cannot pull
            can_pull = false;
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
        "pull" => {
            let local_fork = &topbase_success.fork_point.0.commit.id.hash;
            let take_commits = &topbase_success.top_right_commits;
            try_sync_in(&repo_file, starting_branch_name,
                local_fork, take_commits)
        },
        "push" => {
            let remote_fork = &topbase_success.fork_point.1.commit.id.hash;
            let take_commits = &topbase_success.top_commits;
            try_sync_out(&repo_file, remote_url,
                starting_branch_name, remote_fork, take_commits)
        }

        // this is pull --rebase then push:
        _ => try_sync_in_then_out(),
    }
}

pub fn handle_sync(
    cmd: &MgtCommandSync,
    remote_url: &str,
    repo_file_path: &PathBuf,
    repo_file: &RepoFile,
    sync_type: SyncType,
    topbase_opt: Option<SuccessfulTopbaseResult<CommitWithBlobs>>,
    starting_branch_name: &str,
) -> io::Result<()> {
    match topbase_opt {
        Some(s) => handle_sync2(cmd, remote_url, repo_file_path, repo_file, sync_type, s, starting_branch_name),
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

    // TODO: add a --no-interact mode which would override --ask-branches
    let repo_branch = if cmd.ask_branches {
        let mut desired_branch_choice = interact::InteractChoices::choose_word(
            &format!("What remote branch would you like to fetch? (hit Enter to use {})", repo_branch));
        let description = format!("About to fetch {}", repo_url);
        desired_branch_choice.description = Some(description);
        let desired_branch = interact::interact_word(desired_branch_choice)
            .map_err(|e| ioerr!("Failed to get user's input for a desired remote branch\n{}", e))?;
        let desired_branch = desired_branch.trim_start().trim_end();
        if desired_branch.is_empty() {
            repo_branch.to_string()
        } else {
            desired_branch.to_string()
        }
    } else { repo_branch.to_string() };

    let divider = "=".repeat(15);
    println!("\n{} Fetching {}:{} {}", divider, repo_url, repo_branch, divider);
    git_helpers3::fetch_branch(repo_url, &repo_branch).map_err(|e| ioerr!("{}", e))?;

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
    let (sync_type, topbase_ok) = match topbase_ok {
        None => (SyncType::Disjoint, None),
        Some(o) => {
            // TODO: still not sure if this is how I want to handle
            // merge commit filtering. This is the simplest solution:
            // just dont allow merge commits, and dont show them to the
            // user. Because if we allow a merge commit, then when we
            // do an interactive rebase after filtering, the merge commit
            // will throw off the interactive rebase unless we pass an option
            // that allows them, but what should the desired strategy be?
            // should merge commits just become empty commits after filtering?
            // maybe add an interaction question here to ask the user?
            // i think simply ignoring merge commits is a sensible default.
            // This was originally done by REMOVING all merge commits
            // from this topbase result, but that is not safe because
            // then when we call `try_get_new_commits_after_filter` we are
            // relying on the number of commits being the same, but if
            // we remove merge commits, then that means we could be trying
            // to rebase more commits than we originally wanted to.
            // The solution is to keep the merge commits in the topbase
            // result, BUT DO NOT show it to the user so it doesn't cause
            // any confusion.

            // TODO: can a fork point be a merge commit? I think not, but
            // that could be an issue if that is ever possible.
            let local_empty = o.top_commits.is_empty();
            let remote_empty = o.top_right_commits.is_empty();
            // TODO: handle merge commit filtering here.
            // if user provides --allow-merges option
            // then we consider a merge commit a potential divergence,
            // otherwise, we have to check the bottom case here
            // and if one of the branches only contains merge commits,
            // we pretend they dont exist.
            let sync_type = match (local_empty, remote_empty) {
                (true, true) => SyncType::UpToDate,
                (true, false) => SyncType::RemoteAhead,
                (false, true) => SyncType::LocalAhead,
                (false, false) => SyncType::Diverged,
            };
            (sync_type, Some(o))
        }
    };
    handle_sync(cmd, repo_url, repo_file_path, &repo_file, sync_type, topbase_ok, starting_branch_name)
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
