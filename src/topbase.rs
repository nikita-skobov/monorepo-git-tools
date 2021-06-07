use std::{io, collections::HashSet, process::Stdio, str::FromStr, hash::Hash};
use io::{BufReader, BufRead};

use super::ioerr;
use super::git_helpers3;
use super::git_helpers3::Commit;
use super::git_helpers3::Oid;
use super::git_helpers3::CommitWithBlobs;
use super::git_helpers3::{RawBlobSummaryWithoutPath, RawBlobSummary};
use super::exec_helpers;
use super::die;
use super::cli::MgtCommandTopbase;

/// Determines how blob information should be hashed
/// when conducting a topbase traversal. The default
/// is Full, which is the strictest hashing mode which contains
/// the entire information about the blob. The slightly
/// faster, and sometimes desired mode would be WithoutPath because
/// it considers two blobs the same if everything matches except the path
/// component. A niche hashing mode is
/// EndState: only hash the end state of the blob, ie:
/// if a blob started with file mode A and changed to file mode B,
/// we don't care about the fact it changed from A, we only care that
/// it is currently at B. This needs to be treated carefully in regards
/// to deletions because a deletion SHA goes from X to 000000, and all
/// zeros is of course not unique, and therefore cannot be compared easily.
/// the solution for this is to use the source SHA for deletes
pub enum BlobHashingMode {
    Full,
    WithoutPath,
    EndState,
    EndStateWithoutPath,
}

/// remember, the 'upstream' is the base branch
/// because its the branch that is going to receive commits (if any)
/// and the 'current' branch is the top branch. by default the 'current'
/// branch is also the one that the user is currently on.
pub fn topbase(
    current_branch: String,
    upstream_branch: String,
    dry_run: bool,
    verbose: bool,
    should_add_branch_label: bool,
) -> Result<(), String> {
    let log_p = if dry_run { "   # " } else { "" };

    // we want this ref name to be unambiguous to the get_all_commits
    // command, otherwise it might conflict with a file/folder name
    let current_branch = if current_branch.contains("refs/heads") {
        current_branch
    } else {
        format!("refs/heads/{}", current_branch)
    };

    let mut rebase_data = vec![];
    let num_commits_of_current = match git_helpers3::get_number_of_commits_in_ref(&current_branch) {
        Ok(v) => v,
        Err(e) => die!("Failed to get all commits! {}", e),
    };

    // TODO: make this a cli option:
    let hashing_mode = BlobHashingMode::Full;
    // TODO: make this a cli option:
    let traverse_at_a_time = 500;
    let current_commits_not_in_upstream = find_a_b_difference2(
        &current_branch, &upstream_branch,
        Some(traverse_at_a_time),
        hashing_mode).map_err(|e| e.to_string())?;
    let num_commits_to_take = if let Some(valid_topbase) = current_commits_not_in_upstream {
        let mut num_used = 0;
        for c in &valid_topbase.top_commits {
            if c.is_merge { continue; }
            let rebase_interactive_entry = format!("pick {} {}\n", c.id.long(), c.summary);
            rebase_data.push(rebase_interactive_entry);
            num_used += 1;
        }
        num_used
    } else {
        num_commits_of_current
    };

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
use git_helpers3::{RawBlobSummaryEndStateWithoutPath, RawBlobSummaryEndState};
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

// TODO:
// add fullbase and rewind topbase traversal modes


/// In a Topbase traversal mode, the A branch is considered the 'top', and
/// the B branch is the 'bottom'. In this traversal mode, we load the entire B
/// branch and then traverse the A branch, and we stop as soon as we find a commit
/// in A that exists entirely in B. This means we extract the top commits of A that
/// are not in B. This is fairly efficient when you know that your A branch is most likely
/// simply ahead of B, and B is not ahead of any common fork point of A.
/// On the other hand, if you do not know which branch is ahead (if any), a Fullbase
/// traversal can tell you the entire story between the two branches. It starts by doing
/// a topbase, and finding the fork point. Then it searches up from that fork point
/// on the B branch to see if theres anything in B ahead of the fork point, that is
/// not in A.
/// A hybrid approach is the TopbaseRewind which starts with a Topbase, and then backtracks
/// on the B branch to see if anything *on top* of B has diverged from the top of A.
/// A TopbaseRewind is sufficient for most "I have worked on the main branch for a few days
/// but there might also be work done on the remote main branch since then, so I want
/// to see what kind of merge/rebase I should do".
/// Examples:
/// ```
/// # * denotes a fork point where the blobs match up in the two branches
/// # [0-9] deontes the order of the commits that are traversed
/// # ? denotes a commit that was not traversed, and therefore has no traversal order
/// 
/// # topbase:
///   A1       B?
///   |         |
///   A2*      B?*
///   |         |
///   A?        |
/// # note that below A2 is not traversed because we found A2 to be a common fork point
/// # so we stopped there. also we dont traverse any B commits, because in Topbase, we
/// # only care about finding the different points on the A branch.
/// # doing this topbase would report to the user a scenario of something like:
/// #  A1
/// #   \
/// #    \
/// #     |
/// # (A2 == B?*)
/// #     |
/// #     ?
/// #     ?
///
/// # topbase-rewind:
///   A1       B4
///   |         |
///   A2*     B3*
///   |         |
///   A?        |
/// # in topbase-rewind, we would then switch to the B branch once we found the fork point
/// # in A2, and then we search up, to see if there's anything in B that is not in A.
/// # in this case we found B4 which does not exist in the A branch (that we know of! 
/// # remember, we do not search DOWN in A, so it is possible (but highly unlikely) that
/// # this B4 commit exists somewhere down the A branch), so we can report to the user
/// # that our two branches have diverged something like:
/// #  A1     B4
/// #   \     /
/// #    \  /
/// #     |
/// # (A2 == B3)
/// #     |
/// #     ?
/// #     ?
/// #
/// # fullbase:
///   A1       B4
///   |         |
///   A2*      B5*
///   |         |
///   A3        B6
/// # in topbase, both branches commits/blobs are fully loaded into memory
/// # first, and then we traverse A, and then traverse B. there is nothing smart
/// # about it. It is both slow, and uses a lot of memory, but it is 100% correct.
/// # we find the exact scenario. In this case, we would report to the user:
/// #  A1     B4
/// #   \     /
/// #    \  /
/// #     |
/// # (A2 == B5)
/// #     |
/// #    /\
/// #   /  \
/// #  |    \
/// # A3    B6
/// #
/// # this is quite an odd history, and the user most likely wouldn't want to care about
/// # B6, because the user already synced once before at A2 == B5, but
/// # in some cases, it might be useful to see the entire branch divergence to
/// # get a better picture of the state of everything
/// ```
#[derive(Debug, Copy, Clone)]
pub enum ABTraversalMode {
    Topbase,
    TopbaseRewind,
    Fullbase,
}

impl FromStr for ABTraversalMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ok = match s {
            "topbase" => ABTraversalMode::Topbase,
            "rewind" => ABTraversalMode::TopbaseRewind,
            "fullbase" => ABTraversalMode::Fullbase,
            _ => return Err(format!("{} is not a valid traversal mode", s)),
        };
        Ok(ok)
    }
}

impl Default for ABTraversalMode {
    fn default() -> Self {
        ABTraversalMode::Topbase
    }
}

/// A successful topbase result will find a fork point between
/// two branches A, and B. A is considered the top, and contains
/// a list of commits that are above the common fork point.
pub struct SuccessfulTopbaseResult {
    pub top_commits: Vec<Commit>,
    // its possible that the other branch's commit might have a different
    // message than A's commit, but they share the same blob set.
    // so fork_point.0 is the commit from A, and fork_point.1 is the
    // commit from B.
    pub fork_point: (Commit, Commit),
}

/// A helper struct to manage the iterative loading of commits with blobs
/// from git log. Load N commits at a time every time `load_next()` is called.
/// Load next gets a callback that returns true if you wish to stop reading from the log stream.
#[derive(Debug, Default)]
pub struct BranchIterativeCommitLoader<'a, T: Default + From<RawBlobSummary> + Eq + Hash> {
    pub n: usize,
    pub branch_name: &'a str,
    pub last_commit_id: Option<String>,
    pub commit_set: HashSet<String>,
    pub groups: Vec<Vec<(CommitWithBlobs, HashSet<T>)>>,
    pub entirely_loaded: bool,
    pub should_skip_first_commit: bool,
}

impl<'a, T: Default + From<RawBlobSummary> + Eq + Hash> BranchIterativeCommitLoader<'a, T> {
    pub fn new(n: usize, branch_name: &'a str) -> BranchIterativeCommitLoader<'a, T> {
        let mut out = BranchIterativeCommitLoader::default();
        out.n = n;
        out.branch_name = branch_name;
        out
    }

    pub fn load_next<F>(&mut self, cb: F) -> io::Result<()>
        where F: FnMut((&CommitWithBlobs, &HashSet<T>)) -> bool
    {
        if self.entirely_loaded {
            return Ok(());
        }

        let mut cb = cb;
        let mut this_load_group = vec![];
        let mut is_first_commit = true;
        let use_committish: String = if let Some(ref last_id) = self.last_commit_id {
            last_id.clone()
        } else {
            self.branch_name.to_string()
        };
        // if we are not doing our first load, then we should
        // increase N by 1 so that we skip the previous commit when iterating
        let mut our_first_commit_ever = ! self.should_skip_first_commit;
        let use_n = if self.should_skip_first_commit {
            self.n + 1
        } else {
            self.n
        };
        git_helpers3::iterate_blob_log(&use_committish, Some(use_n), |c| {
            // the first time we run this command, we want to look at this
            // first commit. but every time afterwards, the
            // first commit will be the same as the last commit we looked at
            // so every load after the first: we should skip this commit
            // eprintln!("Loader sees: {} {}", c.commit.id.short(), c.commit.summary);
            if is_first_commit && ! our_first_commit_ever {
                // eprintln!("Loader returning becuase first commit");
                is_first_commit = false;
                if self.commit_set.contains(&c.commit.id.hash) {
                    self.entirely_loaded = true;
                }
                return false;
            } else {
                // TODO: fix this logic... its skipping on the second one for some reason
                our_first_commit_ever = false;
                is_first_commit = false;
            }

            let blob_set: HashSet<T> = c.blobs.iter().cloned().map(|x| {
                T::from(x)
            }).collect();
            let already_seen_commit = self.commit_set.contains(&c.commit.id.hash);
            let ret = if already_seen_commit {
                self.entirely_loaded = true;
                // we can stop loading this stream
                // eprintln!("Loader skipping because already seen it before");
                true
            } else {
                let ret = cb((&c, &blob_set));
                self.commit_set.insert(c.commit.id.hash.clone());
                this_load_group.push((c, blob_set));
                ret
            };
            ret
        })?;

        self.should_skip_first_commit = ! our_first_commit_ever;

        // keep track of the last commit so we can
        // start from here on the next load
        if let Some(last) = this_load_group.last() {
            self.last_commit_id = Some(last.0.commit.id.hash.clone());
        }

        if ! this_load_group.is_empty() {
            self.groups.push(this_load_group);
        }

        Ok(())
    }

    pub fn contains_superset_of(&self, blob_set: &HashSet<T>) -> Option<CommitWithBlobs> {
        for group in self.groups.iter() {
            for (our_commit, our_blob_set) in group.iter() {
                if blob_set.is_subset(our_blob_set) {
                    return Some(our_commit.clone());
                }
            }
        }
        None
    }

    pub fn contains_subset_of(&self, blob_set: &HashSet<T>) -> Option<CommitWithBlobs> {
        for group in self.groups.iter() {
            for (our_commit, our_blob_set) in group.iter() {
                if our_blob_set.is_subset(blob_set) {
                    return Some(our_commit.clone());
                }
            }
        }
        None
    }

    pub fn get_all_above(&mut self, commit_id: &str) -> Option<Vec<Commit>> {
        if ! self.commit_set.contains(commit_id) {
            // we do not have that commit, so we cannot find everything above it...
            return None;
        }

        let mut out_list = vec![];
        for mut group in self.groups.drain(..) {
            for (our_commit, _) in group.drain(..) {
                if our_commit.commit.id.hash == commit_id {
                    return Some(out_list);
                }
                out_list.push(our_commit.commit);
            }
        }

        // even though we checked above for the commit_id,
        // if we fail to find it while iterating that is also an error
        // so we should not return a list of commits
        // unless we guarantee that we found the hash that the user wants
        // TODO: maybe make this a result error?
        None
    }
}

/// this is called by `find_a_b_difference2` if you
/// passed a Some(n) for the traverse n at a time.
pub fn find_a_b_difference2_iterative_traversal<T: Default + From<RawBlobSummary> + Eq + Hash>(
    a_committish: &str, b_committish: &str,
    traverse_n: usize,
) -> io::Result<Option<SuccessfulTopbaseResult>> {
    // first we load N of the B branches commits:
    let mut b_loader = BranchIterativeCommitLoader::<T>::new(traverse_n, b_committish);
    b_loader.load_next(|_| false)?;

    let mut a_loader = BranchIterativeCommitLoader::<T>::new(traverse_n, a_committish);
    let mut found_fork_point = None;

    while ! a_loader.entirely_loaded || ! b_loader.entirely_loaded {
        // we check if A's next blob set is a subset of anything in B we've loaded so far
        a_loader.load_next(|(a_commit, a_blob_set)| {
            if a_commit.commit.is_merge { return false; }
    
            if let Some(b_side_fork) = b_loader.contains_superset_of(a_blob_set) {
                found_fork_point = Some((a_commit.clone(), b_side_fork));
                // true because now that we found our fork point we can stop reading the stream
                true
            } else {
                false
            }
        })?;
    
        if let Some((a_fork, b_fork)) = found_fork_point {
            let top_a_commits = a_loader.get_all_above(&a_fork.commit.id.hash)
                .ok_or(ioerr!("Found a fork point {}, but failed to find commits above it?", a_fork.commit.id.short()))?;
            let successful_topbase = SuccessfulTopbaseResult {
                top_commits: top_a_commits,
                fork_point: (a_fork.commit, b_fork.commit),
            };
            return Ok(Some(successful_topbase));
        }

        // if we failed to find the fork point after searching A's next group,
        // then we load B's next group, and search through all of A:
        b_loader.load_next(|(b_commit, b_blob_set)| {
            if b_commit.commit.is_merge { return false; }

            if let Some(a_side_fork) = a_loader.contains_subset_of(b_blob_set) {
                found_fork_point = Some((a_side_fork, b_commit.clone()));
                // true because now that we found our fork point we can stop reading the stream
                true
            } else {
                false
            }
        })?;

        if let Some((a_fork, b_fork)) = found_fork_point {
            let top_a_commits = a_loader.get_all_above(&a_fork.commit.id.hash)
                .ok_or(ioerr!("Found a fork point {}, but failed to find commits above it?", a_fork.commit.id.short()))?;
            let successful_topbase = SuccessfulTopbaseResult {
                top_commits: top_a_commits,
                fork_point: (a_fork.commit, b_fork.commit),
            };
            return Ok(Some(successful_topbase));
        }
    }

    // if we traversed both A and B and failed to find a fork point, then
    // the topbase is not successful, ie: there is no common fork point
    Ok(None)
}

/// An alternative of `find_a_b_difference` that allows
/// to pass an option of how many commits to look at from each branch
/// at a time. The old way of doing this was to load the entire B branch
/// and then iterate over the A branch. This is very slow for large repos.
/// The point of topbase is to find a recent fork point between two
/// potentially unrelated branches. In other words, we have reason to
/// believe that this fork point is somewhere towards the top of the B branch
/// and we think the A branch is only a little bit ahead of that potentially.
/// so we can maybe avoid loading the entire B branch's commits by iteratively
/// traversing both the A and B branch. The algorithm would work as follows:
/// load N commits from B, then load N commits from A and traverse the A
/// commits. If you find a commit in the first N commits from A that
/// exists somewhere in B, then you are done, that is your fork point.
/// but if you dont find it, then load another N commits from B,
/// AND check the first A commits again. If you failed to find it,
/// then this time load another N from the A branch, AND check all of
/// the B commits that you have loaded so far. keep alternating which
/// branch is loaded as a way to average out the length of time
/// it takes to find the fork point. Worst case is there is no fork
/// point, and you end up traversing both branches anyway, albeit
/// across several git log commands. The worst case here is
/// not that much worse than the old way of doing it where
/// you would just load the entirety of the B branch anyway.
/// A good value of N would probably be around 500-1000.
pub fn find_a_b_difference2(
    a_committish: &str, b_committish: &str,
    traverse_n_at_a_time: Option<usize>,
    // TODO: add traversal mode...
    hashing_mode: BlobHashingMode,
) -> io::Result<Option<SuccessfulTopbaseResult>> {
    if let Some(n) = traverse_n_at_a_time {
        // 0 is not a valid value of N
        if n == 0 {
            return simplest_topbase(a_committish, b_committish, hashing_mode);
        }

        match hashing_mode {
            BlobHashingMode::Full => find_a_b_difference2_iterative_traversal::<RawBlobSummary>(
                a_committish, b_committish, n),
            BlobHashingMode::WithoutPath => find_a_b_difference2_iterative_traversal::<RawBlobSummaryWithoutPath>(
                a_committish, b_committish, n),
            BlobHashingMode::EndState => find_a_b_difference2_iterative_traversal::<RawBlobSummaryEndState>(
                a_committish, b_committish, n),
            BlobHashingMode::EndStateWithoutPath => find_a_b_difference2_iterative_traversal::<RawBlobSummaryEndStateWithoutPath>(
                a_committish, b_committish, n),
        }
    } else {
        simplest_topbase(a_committish, b_committish, hashing_mode)
    }
}

pub fn simplest_topbase_inner<T: From<RawBlobSummary> + Eq + Hash>(
    a_committish: &str, b_committish: &str,
) -> io::Result<Option<SuccessfulTopbaseResult>> {
    let mut all_b_commits = vec![];
    git_helpers3::iterate_blob_log(b_committish, None, |c| {
        let b_blob_set: HashSet<T> = c.blobs.iter().cloned().map(|x| {
            T::from(x)
        }).collect();
        all_b_commits.push((c, b_blob_set));
        false
    })?;

    let mut top_a_commits = vec![];
    let mut fork_point = None;
    git_helpers3::iterate_blob_log(a_committish, None, |c| {
        let mut c = c;
        let a_blob_set: HashSet<T> = c.blobs.drain(..).map(|x| {
            T::from(x)
        }).collect();
        let mut found_matching_commit_in_b = None;
        for (b_commit, b_blob_set) in all_b_commits.iter() {
            if c.commit.is_merge {
                // if its a merge commit, then (unless we passed the -m option to git log)
                // it will have no blobs, and thus an empty set is a subset of any other set.
                // so we dont want to check this...
                break;
            }
            if a_blob_set.is_subset(b_blob_set) {
                found_matching_commit_in_b = Some(b_commit);
                break;
            }
            // TODO: what about checking if B's blob set is a subset of A's set?
            // I think that would be the most 'correct'
            // TODO: also remember if you implement checking for B's subsets as well,
            // then need to remember some of B's blob sets might be empty
            // because they are merge commits
        }

        if let Some(matching) = found_matching_commit_in_b {
            fork_point = Some((c, matching));
            // in simple topbase, once we found the fork point,
            // we can return and stop reading the git log stream:
            return true;
        } else {
            top_a_commits.push(c);
        }

        false
    })?;

    if let Some((fork_a, fork_b)) = fork_point {
        let successful_topbase = SuccessfulTopbaseResult {
            top_commits: top_a_commits.drain(..).map(|x| x.commit).collect(),
            fork_point: (fork_a.commit, fork_b.commit.clone()),
        };
        Ok(Some(successful_topbase))
    } else {
        // failed to find a fork point
        Ok(None)
    }
}

/// in the simplest way to topbase, we load entirety
/// of B branch into memory, then iterate
/// over the A branch, and stop once we find
/// a commit that is a subset of one of the B branch's commits.
/// see `BlobHashingMode` docs for information about which hashing mode to use.
/// When in doubt, use Full for maximum correctness, or EndState for maximum flexibility.
pub fn simplest_topbase(
    a_committish: &str, b_committish: &str,
    hashing_mode: BlobHashingMode,
) -> io::Result<Option<SuccessfulTopbaseResult>> {
    match hashing_mode {
        BlobHashingMode::Full => simplest_topbase_inner::<RawBlobSummary>(
            a_committish, b_committish),
        BlobHashingMode::WithoutPath => simplest_topbase_inner::<RawBlobSummaryWithoutPath>(
            a_committish, b_committish),
        BlobHashingMode::EndState => simplest_topbase_inner::<RawBlobSummaryEndState>(
            a_committish, b_committish),
        BlobHashingMode::EndStateWithoutPath => simplest_topbase_inner::<RawBlobSummaryEndStateWithoutPath>(
            a_committish, b_committish),
    }
}


// TODO:
// - rewrite the blob set to be a blob map. need a way to find which
// commit a blob points to...

#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;

    #[test]
    fn loader_doesnt_make_more_groups_than_commit_count() {
        // we pass a very high value of N, if we call
        // load_next() several times, there should still only be 1 group
        // TODO: if this repo ever gets more than 1000000 commits, update this:
        let mut loader = BranchIterativeCommitLoader::<RawBlobSummary>::new(1000000, "HEAD");
        loader.load_next(|_| false).unwrap();
        assert_eq!(loader.groups.len(), 1);
        let num_commits = loader.groups[0].len();
        loader.load_next(|_| false).unwrap();
        loader.load_next(|_| false).unwrap();
        loader.load_next(|_| false).unwrap();
        assert_eq!(loader.groups.len(), 1);
        assert_eq!(loader.groups[0].len(), num_commits);
    }

    // TODO: this test wont work in github pipeline
    // because I think github downloads only one commit, so
    // it fails to load the second time.. sad
    // easiest fix is probably to change the github pipeline to load
    // the entire repo, not just latest commit
    // #[test]
    fn loader_groups_are_disjoint() {
        let mut loader = BranchIterativeCommitLoader::<RawBlobSummary>::new(1, "HEAD");
        loader.load_next(|_| false).unwrap();
        assert_eq!(loader.groups.len(), 1);
        assert_eq!(loader.groups[0].len(), 1);
        let first_commit = loader.groups[0][0].0.commit.id.hash.clone();
        loader.load_next(|_| false).unwrap();
        assert_eq!(loader.groups.len(), 2);
        assert_eq!(loader.groups[0].len(), 1);
        assert_eq!(loader.groups[1].len(), 1);
        let next_commit = loader.groups[1][0].0.commit.id.hash.clone();
        assert!(first_commit != next_commit);
    }
}
