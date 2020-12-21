use std::collections::HashSet;

use super::git_helpers3;
use super::git_helpers3::Commit;
use super::exec_helpers;
use super::check::topbase_check_alg;
use super::die;
use super::cli::MgtCommandTopbase;

/// remember, the 'upstream' is the base branch
/// because its the branch that is going to receive commits (if any)
/// and the 'current' branch is the top branch. by default the 'current'
/// branch is also the one that the user is currently on.
fn topbase(
    current_branch: String,
    upstream_branch: String,
    dry_run: bool,
    verbose: bool,
    should_add_branch_label: bool,
) -> Result<(), String> {
    let log_p = if dry_run { "   # " } else { "" };

    let all_upstream_blobs = get_all_blobs_in_branch(upstream_branch.as_str());
    let all_commits_of_current = match git_helpers3::get_all_commits_from_ref(current_branch.as_str()) {
        Ok(v) => v,
        Err(e) => die!("Failed to get all commits! {}", e),
    };

    let num_commits_of_current = all_commits_of_current.len();
    let mut num_commits_to_take = 0;
    let mut rebase_data = vec![];
    let mut cb = |c: &Commit| {
        num_commits_to_take += 1;
        let rebase_interactive_entry = format!("pick {} {}\n", c.id.long(), c.summary);
        rebase_data.push(rebase_interactive_entry);
    };
    topbase_check_alg(all_commits_of_current, all_upstream_blobs, &mut cb);

    // need to reverse it because git rebase interactive
    // takes commits in order of oldest to newest, but
    // we parsed them from newest to oldest
    rebase_data.reverse();

    // we just want to use the actual branch names, not the ref name
    let current_branch = current_branch.replace("refs/heads/", "");
    let upstream_branch = upstream_branch.replace("refs/heads/", "");

    // if nothing to take, dont topbase
    // instead go back to upstream, and then
    // delete delete the current branch
    if num_commits_to_take == 0 {
        if dry_run {
            println!("{}Nothing to topbase. Returning to {}", log_p, upstream_branch);
            println!("{}Deleting {}", log_p, current_branch);
            return Ok(());
        }

        println!("Nothing to topbase. Returning to {}", upstream_branch);
        match git_helpers3::checkout_branch(upstream_branch.as_str(), false) {
            Err(e) => die!("Failed to checkout back to upstream branch: {}", e),
            _ => (),
        }
        println!("Deleting {}", current_branch);
        match git_helpers3::delete_branch(current_branch.as_str()) {
            Err(e) => die!("Failed to delete temporary branch {}: {}", current_branch, e),
            _ => (),
        }

        return Ok(());
    }

    // if we need to topbase the entirety of the current branch
    // it will be better to do a regular rebase
    let args = if num_commits_to_take == num_commits_of_current {
        // if we are trying to topbase on a branch that hasnt been rebased yet,
        // we dont need to topbase, and instead we need to do a regular rebase
        println!("{}no commit of {} exists in {}. rebasing non-interactively", log_p, current_branch, upstream_branch);

        let args = vec![
            "git".into(), "rebase".into(), upstream_branch.clone(),
        ];
        args
    } else {
        vec![]
    };

    // args will have non-zero length only if
    // we need to topbase all commits
    if args.len() != 0 {
        if dry_run {
            let arg_str = args.join(" ");
            println!("{}", arg_str);
            return Ok(());
        }

        let str_args: Vec<&str> = args.iter().map(|f| f.as_str()).collect();
        let err_msg = match exec_helpers::execute(
            &str_args[..]
        ) {
            Err(e) => Some(vec![format!("{}", e)]),
            Ok(o) => {
                match o.status {
                    0 => None,
                    _ => Some(vec![o.stderr.lines().next().unwrap().to_string()]),
                }
            },
        };
        if let Some(err) = err_msg {
            let err_details = match verbose {
                true => format!("{}", err.join("\n")),
                false => "".into(),
            };
            println!("Failed to rebase\n{}", err_details);
            return Err(err_details);
        }

        return Ok(());
    }

    if dry_run || verbose {
        // since we are already on the rebase_from_branch
        // we dont need to specify that in the git command
        // the below command implies: apply rebased changes in
        // the branch we are already on
        println!("rebase_data=\"{}\"", rebase_data.join(""));
        println!("GIT_SEQUENCE_EDITOR=\"echo $rebase_data >\" git rebase -i --onto {} {}~{} {}",
            upstream_branch,
            current_branch,
            num_commits_to_take,
            current_branch,
        );
        if dry_run {
            return Ok(());
        }
    }

    // only add label in certain circumstances,
    // otherwise a label being added is unnecessary
    // and annoying
    if should_add_branch_label {
        // if we've made it this far, that
        // means we have commits to topbase
        // so we should add a label here of the upstream
        // branch, so if the user does a git log after topbase
        // they can visualize which commits were added on top
        let label_name = format!("{}-remote", current_branch);
        let _ = exec_helpers::execute(&["git", "branch", label_name.as_str(), upstream_branch.as_str()]);
    }

    // rebase_data="pick <hash> <msg>
    // pick <hash> <msg>
    // pick <hash> <msg>
    // "
    // rebase_command="echo \"$rebase_data\""
    // GIT_SEQUENCE_EDITOR="$rebase_command >" git rebase -i --onto bottom top~3 top
    let upstream_arg = format!("{}~{}", current_branch, num_commits_to_take);
    let args = [
        "git", "rebase", "-i",
        "--onto", upstream_branch.as_str(),
        upstream_arg.as_str(),
        current_branch.as_str(),
    ];
    let rebase_data_str = rebase_data.join("");
    let rebase_data_str = format!("echo \"{}\" >", rebase_data_str);

    let err_msg = match exec_helpers::execute_with_env(
        &args,
        &["GIT_SEQUENCE_EDITOR"],
        &[rebase_data_str.as_str()],
    ) {
        Err(e) => Some(vec![format!("{}", e)]),
        Ok(o) => {
            match o.status {
                0 => None,
                _ => Some(vec![o.stderr.lines().next().unwrap().to_string()]),
            }
        },
    };

    if let Some(err) = err_msg {
        let err_details = match verbose {
            true => format!("{}", err.join("\n")),
            false => "".into(),
        };
        println!("Failed to rebase\n{}", err_details);
        Err(err_details)
    } else {
        Ok(())
    }
}

pub enum BlobCheckValue {
    TakeNext,
    TakePrev,
}
use BlobCheckValue::*;
pub struct BlobCheck<'a> {
    pub mode_prev: &'a str,
    pub mode_next: &'a str,
    pub blob_prev: &'a str,
    pub blob_next: &'a str,
    pub path: String,
}

pub fn blob_check_callback_default(blob_check: &BlobCheck) -> Option<BlobCheckValue> {
    match blob_check.is_delete_blob() {
        true => Some(TakePrev),
        false => Some(TakeNext),
    }
}

impl<'a> BlobCheck<'a> {
    fn is_delete_blob(&self) -> bool {
        let blob_prev_not_all_zeroes = ! self.blob_prev.chars().all(|c| c == '0');
        let blob_next_all_zeroes = self.blob_next.chars().all(|c| c == '0');
        blob_next_all_zeroes && blob_prev_not_all_zeroes
    }
}

// run a git diff-tree on the commit id, and parse the output
// and for every blob, if callback returns true,
// insert that blob id into the provided blob hash set
pub fn get_all_blobs_from_commit_with_callback(
    commit_id: &str,
    blob_set: &mut HashSet<String>,
    insert_callback: Option<&dyn Fn(&BlobCheck) -> Option<BlobCheckValue>>,
) {
    // the diff filter is VERY important...
    // A (added), M (modified), C (copied), D (deleted)
    // theres a few more..
    let args = [
        "git", "diff-tree", commit_id, "-r", "--root",
        "--diff-filter=AMCD", "--pretty=oneline"
    ];
    match exec_helpers::execute(&args) {
        Err(e) => die!("Failed to get blobs from commit {} : {}", commit_id, e),
        Ok(out) => {
            if out.status != 0 { die!("Failed to get blobs from commit {} : {}", commit_id, out.stderr); }
            for l in out.stdout.lines() {
                // lines starting with colons are the lines
                // that contain blob ids
                if ! l.starts_with(':') { continue; }
                let items = l.split_whitespace().collect::<Vec<&str>>();
                // there are technically 6 items from this output:
                // the last item (items[5]) is a path to the file that this blob
                // is for (and the array could have more than 6 if file names
                // have spaces in them)
                let (
                    mode_prev, mode_next,
                    blob_prev, blob_next,
                    _diff_type
                ) = (items[0], items[1], items[2], items[3], items[4]);
                // the path of this blob starts at index 5, but we combine the rest
                // in case there are spaces
                let blob_path = items[5..items.len()].join(" ");
                let blob_check = BlobCheck {
                    mode_prev,
                    mode_next,
                    blob_prev,
                    blob_next,
                    path: blob_path,
                };
                // if user provided a callback, ask the user A) if they want to take this
                // blob, and B) which one to take (next or prev)
                // otherwise, use the default way to decide which one to take
                let should_take = match insert_callback {
                    Some(ref which_to_take_callback) => which_to_take_callback(&blob_check),
                    None => blob_check_callback_default(&blob_check),
                };
                if let Some(which) = should_take {
                    match which {
                        TakeNext => blob_set.insert(blob_next.into()),
                        TakePrev => blob_set.insert(blob_prev.into()),
                    };
                }
            }
        }
    };
}

pub fn get_all_blobs_from_commit<'a>(
    commit_id: &str,
    blob_set: &mut HashSet<String>,
) {
    get_all_blobs_from_commit_with_callback(
        commit_id,
        blob_set,
        None,
    );
}

// perform a rev-list of the branch name to get a list of all commits
// then get every single blob from every single commit, and return
// a hash set containing unique blob ids
pub fn get_all_blobs_in_branch(branch_name: &str) -> HashSet<String> {
    // first get all commits from this branch:
    let args = [
        "git", "rev-list", branch_name,
    ];

    // need the stdout to live outside the match so that the vec of strings
    // lives outside the match
    let mut _out_stdout = "".into();
    let commit_ids = match exec_helpers::execute(&args) {
        Err(e) => die!("Failed to get all blobs of {} : {}", branch_name, e),
        Ok(out) => {
            if out.status != 0 { die!("Failed to get all blobs of {} : {}", branch_name, out.stderr); }
            _out_stdout = out.stdout;
            _out_stdout.split_whitespace().collect::<Vec<&str>>()
        },
    };

    let mut blob_set = HashSet::new();
    for commit_id in commit_ids.iter() {
        get_all_blobs_from_commit(commit_id, &mut blob_set);
    }

    return blob_set;
}

pub fn run_topbase(cmd: &mut MgtCommandTopbase) {
    let (base, top) = match cmd.base_or_top.len() {
        0 => die!("Must provide a base branch"),
        1 => (cmd.base_or_top[0].clone(), get_current_branch()),
        2 => (cmd.base_or_top[0].clone(), cmd.base_or_top[1].clone()),
        x => die!("You provided {} branch labels, but the max is 2", x),
    };

    // for the topbase command, adding a branch label
    // doesnt make sense. its only used for split-out
    let should_add_branch_label = false;
    let res = topbase(
        top,
        base,
        cmd.dry_run,
        cmd.verbose,
        should_add_branch_label
    );
    if let Err(e) = res {
        die!("Failed to topbase: {}", e);
    }
}

fn get_current_branch() -> String {
    match git_helpers3::get_current_ref() {
        Ok(s) => s,
        Err(e) => die!("Failed to find current git branch: {}", e),
    }
}
