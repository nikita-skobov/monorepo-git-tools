use clap::ArgMatches;
use std::collections::HashSet;

use super::git_helpers3;
use super::git_helpers3::Commit;
use super::git_helpers3::Oid;
use super::exec_helpers;
use super::split::Runner;
use super::check::topbase_check_alg;
use super::commands::TOPBASE_CMD_BASE;
use super::commands::TOPBASE_CMD_TOP;
use super::commands::VERBOSE_ARG;
use super::commands::DRY_RUN_ARG;
use super::die;

pub trait Topbase {
    fn topbase(self) -> Self;
}

impl<'a> Topbase for Runner<'a> {
    fn topbase(mut self) -> Self {
        // for split commands, we always use current ref,
        // but for topbase command, we check if user provided a top branch
        // if user provided one, we use that, otherwise we use current
        let current_branch = if let Some(ref b) = self.topbase_top_ref {
            b.clone()
        } else {
            match git_helpers3::get_current_ref() {
                Ok(s) => s,
                Err(_) => {
                    println!("Failed to get current branch. not going to rebase");
                    return self;
                },
            }
        };

        // upstream is base
        let upstream_branch = match self.repo_original_ref {
            Some(ref branch) => branch.clone(),
            None => {
                println!("Failed to get repo original ref. Not going to rebase");
                return self;
            },
        };

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
            if self.dry_run {
                println!("{}Nothing to topbase. Returning to {}", self.log_p, upstream_branch);
                println!("{}Deleting {}", self.log_p, current_branch);
                return self;
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

            return self;
        }

        // if we need to topbase the entirety of the current branch
        // it will be better to do a regular rebase
        let args = if num_commits_to_take == num_commits_of_current {
            // if we are trying to topbase on a branch that hasnt been rebased yet,
            // we dont need to topbase, and instead we need to do a regular rebase
            println!("{}no commit of {} exists in {}. rebasing non-interactively", self.log_p, current_branch, upstream_branch);

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
            if self.dry_run {
                let arg_str = args.join(" ");
                println!("{}", arg_str);
                return self;
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
                self.status = 1;
                let err_details = match self.verbose {
                    true => format!("{}", err.join("\n")),
                    false => "".into(),
                };
                println!("Failed to rebase\n{}", err_details);
            }
            return self;
        }

        if self.dry_run || self.verbose {
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
            if self.dry_run {
                return self;
            }
        }

        // only add label in certain circumstances,
        // otherwise a label being added is unnecessary
        // and annoying
        if self.topbase_add_label {
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
            self.status = 1;
            let err_details = match self.verbose {
                true => format!("{}", err.join("\n")),
                false => "".into(),
            };
            println!("Failed to rebase\n{}", err_details);
        }
        self
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
                    diff_type
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
    let mut out_stdout = "".into();
    let commit_ids = match exec_helpers::execute(&args) {
        Err(e) => die!("Failed to get all blobs of {} : {}", branch_name, e),
        Ok(out) => {
            if out.status != 0 { die!("Failed to get all blobs of {} : {}", branch_name, out.stderr); }
            out_stdout = out.stdout;
            out_stdout.split_whitespace().collect::<Vec<&str>>()
        },
    };

    let mut blob_set = HashSet::new();
    for commit_id in commit_ids.iter() {
        get_all_blobs_from_commit(commit_id, &mut blob_set);
    }

    return blob_set;
}

pub fn run_topbase(matches: &ArgMatches) {
    // should be safe to unwrap because its a required argument
    let base_branch = matches.value_of(TOPBASE_CMD_BASE).unwrap();
    let top_branch = matches.value_of(TOPBASE_CMD_TOP);
    let mut runner = Runner::new(matches);
    // repo_original_ref is used by the other commands (splitout/splitin)
    // but for topbase this is really the base branch
    runner.repo_original_ref = Some(base_branch.into());
    // if user didnt provide top branch, topbase_top_ref stays None
    // and then the runner.topbase will just use the current branch
    if let Some(t) = top_branch {
        runner.topbase_top_ref = Some(t.to_string());
    }

    runner.save_current_dir()
        .get_repository_from_current_dir()
        .topbase();
}
