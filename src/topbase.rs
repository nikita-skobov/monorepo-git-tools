use std::{io, collections::HashSet, process::Stdio, str::FromStr, hash::Hash, fmt::Debug};
use io::{BufReader, BufRead};

use super::ioerr;
use super::git_helpers3;
use super::git_helpers3::Commit;
use super::git_helpers3::CommitWithBlobs;
use super::git_helpers3::{RawBlobSummaryWithoutPath, RawBlobSummary};
use super::git_helpers3::{RawBlobSummaryEndStateWithoutPath, RawBlobSummaryEndState};
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
    // topbasing pretty much always needs to be end state, doesnt it?
    // because in the case of merge commits you can have
    // a case where the sha on the base branch is X -> Y,
    // but on the top branch it is 0 -> Y, and if we dont
    // use EndState, then we detect those blobs as
    // being different because they had different origins...
    // I think the only hashing mode we should care about
    // would be end state.
    let hashing_mode = BlobHashingMode::EndState;
    // TODO: make this a cli option:
    let traverse_at_a_time = 500;
    let current_commits_not_in_upstream = find_a_b_difference2::<Commit, NopCB>(
        &current_branch, &upstream_branch,
        Some(traverse_at_a_time),
        hashing_mode, false, None).map_err(|e| e.to_string())?;
    let num_commits_to_take = if let Some(valid_topbase) = current_commits_not_in_upstream {
        let mut num_used = 0;
        for c in &valid_topbase.top_commits {
            // sensible default is to not include merge commits
            // when topbasing. TODO: make this a cli option
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
// add fullbase traversal mode implementation


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
pub struct SuccessfulTopbaseResult<T: From<CommitWithBlobs>> {
    pub top_commits: Vec<T>,
    // its possible that the other branch's commit might have a different
    // message than A's commit, but they share the same blob set.
    // so fork_point.0 is the commit from A, and fork_point.1 is the
    // commit from B.
    pub fork_point: (T, T),
    // only used for rewind topbase
    pub top_right_commits: Vec<T>,
}

/// A helper struct to manage the iterative loading of commits with blobs
/// from git log. Load N commits at a time every time `load_next()` is called.
/// Load next gets a callback that returns true if you wish to stop reading from the log stream.
#[derive(Debug, Default)]
pub struct BranchIterativeCommitLoader<'a, T: Debug + Default + From<RawBlobSummary> + Eq + Hash> {
    pub n: usize,
    pub branch_name: &'a str,
    pub last_commit_id: Option<String>,
    pub commit_set: HashSet<String>,
    pub groups: Vec<Vec<(CommitWithBlobs, HashSet<T>)>>,
    pub entirely_loaded: bool,
    pub should_skip_first_commit: bool,
}

impl<'a, T: Debug + Default + From<RawBlobSummary> + Eq + Hash> BranchIterativeCommitLoader<'a, T> {
    pub fn new(n: usize, branch_name: &'a str) -> BranchIterativeCommitLoader<'a, T> {
        let mut out = BranchIterativeCommitLoader::default();
        out.n = n;
        out.branch_name = branch_name;
        out
    }

    pub fn load_next<F, B>(
        &mut self,
        should_use_blob: &mut B,
        cb: F
    ) -> io::Result<()>
        where F: FnMut((&CommitWithBlobs, &HashSet<T>)) -> bool,
        B: FnMut(&mut RawBlobSummary, &str) -> bool,
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
                our_first_commit_ever = false;
                is_first_commit = false;
            }

            let mut before_hash = vec![];
            for mut blob in c.blobs.iter().cloned() {
                if should_use_blob(&mut blob, self.branch_name) {
                    before_hash.push(blob);
                }
            }
            // if we filter out
            // all of the blobs, then dont
            // add this commit because its blob_set will be empty,
            // therefore it will be a subset of any other set,
            // and will mess up the fork point detection.
            // if its a merge commit, its blob set will be empty, so
            // we can insert this because sometimes we might want to look at merge
            // commits
            // TODO: should find a better way of not allowing
            // empty sets to be detected as subsets. this logic of
            // not inserting commits because they dont have blobs
            // might cause issues in the future
            let can_insert_commit = c.commit.is_merge || ! before_hash.is_empty();
            let blob_set: HashSet<T> = before_hash.drain(..).map(|x| {
                T::from(x)
            }).collect();
            let already_seen_commit = self.commit_set.contains(&c.commit.id.hash);
            let ret = if already_seen_commit {
                self.entirely_loaded = true;
                // we can stop loading this stream
                // eprintln!("Loader skipping because already seen it before");
                true
            } else {
                self.commit_set.insert(c.commit.id.hash.clone());
                let ret = if can_insert_commit {
                    let ret = cb((&c, &blob_set));
                    this_load_group.push((c, blob_set));
                    ret
                } else { false };
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

    pub fn get_all_above<C: From<CommitWithBlobs>>(
        &mut self,
        commit_id: &str
    ) -> Option<Vec<C>> {
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
                out_list.push(C::from(our_commit));
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

impl From<CommitWithBlobs> for Commit {
    fn from(orig: CommitWithBlobs) -> Self {
        orig.commit
    }
}

pub fn get_rewind_commits_from_loader<
    T: Debug + Default + From<RawBlobSummary> + Eq + Hash,
    C: From<CommitWithBlobs>
>(
    right_side_loader: &mut BranchIterativeCommitLoader<T>,
    fork_hash: &str,
    should_rewind: bool,
) -> Vec<C> {
    let out_opt = if should_rewind {
        right_side_loader.get_all_above(fork_hash)
    } else {
        None
    };
    match out_opt {
        Some(v) => v,
        None => vec![],
    }
}

/// this is called by `find_a_b_difference2` if you
/// passed a Some(n) for the traverse n at a time.
pub fn find_a_b_difference2_iterative_traversal<
    T: Debug + Default + From<RawBlobSummary> + Eq + Hash,
    C: From<CommitWithBlobs>,
    B: FnMut(&mut RawBlobSummary, &str) -> bool,
>(
    a_committish: &str, b_committish: &str,
    traverse_n: usize,
    should_rewind: bool,
    should_use_blob_cb: B,
) -> io::Result<Option<SuccessfulTopbaseResult<C>>> {
    let mut should_use_blob_cb = should_use_blob_cb;
    // first we load N of the B branches commits:
    let mut b_loader = BranchIterativeCommitLoader::<T>::new(traverse_n, b_committish);
    b_loader.load_next(&mut should_use_blob_cb, |_| false)?;

    let mut a_loader = BranchIterativeCommitLoader::<T>::new(traverse_n, a_committish);
    let mut found_fork_point = None;

    while ! a_loader.entirely_loaded || ! b_loader.entirely_loaded {
        // we check if A's next blob set is a subset of anything in B we've loaded so far
        a_loader.load_next(&mut should_use_blob_cb, |(a_commit, a_blob_set)| {
            if let Some(b_side_fork) = b_loader.contains_superset_of(a_blob_set) {
                found_fork_point = Some((a_commit.clone(), b_side_fork));
                // true because now that we found our fork point we can stop reading the stream
                true
            } else {
                false
            }
        })?;

        if let Some((a_fork, b_fork)) = found_fork_point {
            let top_a_commits = a_loader.get_all_above::<C>(&a_fork.commit.id.hash)
                .ok_or(ioerr!("Found a fork point {}, but failed to find commits above it?", a_fork.commit.id.short()))?;
            let successful_topbase = SuccessfulTopbaseResult {
                top_commits: top_a_commits,
                top_right_commits: get_rewind_commits_from_loader(&mut b_loader, &b_fork.commit.id.hash, should_rewind),
                fork_point: (a_fork.into(), b_fork.into()),
            };
            return Ok(Some(successful_topbase));
        }

        // if we failed to find the fork point after searching A's next group,
        // then we load B's next group, and search through all of A:
        b_loader.load_next(&mut should_use_blob_cb, |(b_commit, b_blob_set)| {
            if let Some(a_side_fork) = a_loader.contains_subset_of(b_blob_set) {
                found_fork_point = Some((a_side_fork, b_commit.clone()));
                // true because now that we found our fork point we can stop reading the stream
                true
            } else {
                false
            }
        })?;

        if let Some((a_fork, b_fork)) = found_fork_point {
            let top_a_commits = a_loader.get_all_above::<C>(&a_fork.commit.id.hash)
                .ok_or(ioerr!("Found a fork point {}, but failed to find commits above it?", a_fork.commit.id.short()))?;
            let successful_topbase = SuccessfulTopbaseResult {
                top_commits: top_a_commits,
                top_right_commits: get_rewind_commits_from_loader(&mut b_loader, &b_fork.commit.id.hash, should_rewind),
                fork_point: (a_fork.into(), b_fork.into()),
            };
            return Ok(Some(successful_topbase));
        }
    }

    // if we traversed both A and B and failed to find a fork point, then
    // the topbase is not successful, ie: there is no common fork point
    Ok(None)
}

pub type NopCB = fn(&mut RawBlobSummary, &str) -> bool;

pub fn find_a_b_difference2_iterative_traversal_opt<
    T: Debug + Default + From<RawBlobSummary> + Eq + Hash,
    C: From<CommitWithBlobs>,
    B: FnMut(&mut RawBlobSummary, &str) -> bool,
>(
    a_committish: &str, b_committish: &str,
    traverse_n: usize,
    should_rewind: bool,
    should_use_blob_cb: Option<B>,
) -> io::Result<Option<SuccessfulTopbaseResult<C>>> {
    if let Some(cb) = should_use_blob_cb {
        return find_a_b_difference2_iterative_traversal::<T, C, B>(a_committish, b_committish, traverse_n, should_rewind, cb);
    }
    
    let default_cb = |_: &mut RawBlobSummary, _: &str| true;
    find_a_b_difference2_iterative_traversal::<T, C, NopCB>(a_committish, b_committish, traverse_n, should_rewind, default_cb)
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
pub fn find_a_b_difference2<
    C: From<CommitWithBlobs>,
    B: FnMut(&mut RawBlobSummary, &str) -> bool,
>(
    a_committish: &str, b_committish: &str,
    traverse_n_at_a_time: Option<usize>,
    // TODO: add traversal mode...
    hashing_mode: BlobHashingMode,
    should_rewind: bool,
    should_use_blob_cb: Option<B>,
) -> io::Result<Option<SuccessfulTopbaseResult<C>>> {
    if let Some(n) = traverse_n_at_a_time {
        // 0 is not a valid value of N
        if n == 0 {
            return simplest_topbase(a_committish, b_committish, hashing_mode);
        }

        match hashing_mode {
            BlobHashingMode::Full => find_a_b_difference2_iterative_traversal_opt::<RawBlobSummary, C, _>(
                a_committish, b_committish, n, should_rewind, should_use_blob_cb),
            BlobHashingMode::WithoutPath => find_a_b_difference2_iterative_traversal_opt::<RawBlobSummaryWithoutPath, C, _>(
                a_committish, b_committish, n, should_rewind, should_use_blob_cb),
            BlobHashingMode::EndState => find_a_b_difference2_iterative_traversal_opt::<RawBlobSummaryEndState, C, _>(
                a_committish, b_committish, n, should_rewind, should_use_blob_cb),
            BlobHashingMode::EndStateWithoutPath => find_a_b_difference2_iterative_traversal_opt::<RawBlobSummaryEndStateWithoutPath, C, _>(
                a_committish, b_committish, n, should_rewind, should_use_blob_cb),
        }
    } else {
        simplest_topbase(a_committish, b_committish, hashing_mode)
    }
}

pub fn simplest_topbase_inner<
    T: From<RawBlobSummary> + Eq + Hash,
    C: From<CommitWithBlobs>,
    B: FnMut(&mut RawBlobSummary, &str) -> bool,
>(
    a_committish: &str, b_committish: &str,
    should_rewind: bool,
    should_use_blob_cb: Option<B>,
) -> io::Result<Option<SuccessfulTopbaseResult<C>>> {
    let mut should_use_blob_cb = should_use_blob_cb;
    let mut all_b_commits = vec![];
    git_helpers3::iterate_blob_log(b_committish, None, |c| {
        let b_blob_set = if let Some(ref mut cb) = should_use_blob_cb {
            // if user provided a callback, then only include this blob
            // in the hash set if the user wants this to be included
            let mut before_hash = vec![];
            for mut blob in c.blobs.iter().cloned() {
                if cb(&mut blob, b_committish) {
                    before_hash.push(blob);
                }
            }
            let b_blob_set: HashSet<T> = before_hash.drain(..).map(|x| {
                T::from(x)
            }).collect();
            b_blob_set
        } else {
            // otherwise, just hash them all
            let b_blob_set: HashSet<T> = c.blobs.iter().cloned().map(|x| {
                T::from(x)
            }).collect();
            b_blob_set
        };
        let can_insert_commit = c.commit.is_merge || ! b_blob_set.is_empty();
        // only add it to B's commits if 
        if can_insert_commit {
            all_b_commits.push((c, b_blob_set));
        }
        false
    })?;

    let mut top_a_commits = vec![];
    let mut fork_point = None;
    git_helpers3::iterate_blob_log(a_committish, None, |c| {
        let mut c = c;
        let a_blob_set = if let Some(ref mut cb) = should_use_blob_cb {
            let mut before_hash = vec![];
            for mut blob in c.blobs.drain(..) {
                if cb(&mut blob, a_committish) {
                    before_hash.push(blob);
                }
            }
            let a_blob_set: HashSet<T> = before_hash.drain(..).map(|x| {
                T::from(x)
            }).collect();
            a_blob_set
        } else {
            let a_blob_set: HashSet<T> = c.blobs.drain(..).map(|x| {
                T::from(x)
            }).collect();
            a_blob_set
        };
        let mut found_matching_commit_in_b = None;
        for (b_commit, b_blob_set) in all_b_commits.iter() {
            if a_blob_set.is_empty() {
                // if its empty that means its either a merge commit
                // or the user filtered out all of the blobs, ie:
                // user doesnt want to consider this commit because
                // none of the blobs apply to what the user is looking for.
                // in this case, we dont want to try to search
                // for a fork point for this commit.
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
        // if we should rewind, then we iterate over the B commits
        // and collect everything above B's fork point:
        let b_above_fork = if should_rewind {
            let mut out = vec![];
            for (b_commit, _) in all_b_commits.iter() {
                if b_commit.commit.id.hash == fork_b.commit.id.hash {
                    // found the fork point, dont add this one
                    break;
                }
                out.push(b_commit.clone().into());
            }
            // TODO: technically this is not guaranteed that we reach
            // this fork point?
            // todo think of data structure where we can guarantee if
            // we found the fork point earlier that we can backtrack from
            // that fork point and guarantee that those commits exists
            out
        } else {
            vec![]
        };
        let successful_topbase = SuccessfulTopbaseResult {
            top_commits: top_a_commits.drain(..).map(|x| x.into()).collect(),
            fork_point: (fork_a.into(), fork_b.clone().into()),
            top_right_commits: b_above_fork,
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
pub fn simplest_topbase<C: From<CommitWithBlobs>>(
    a_committish: &str, b_committish: &str,
    hashing_mode: BlobHashingMode,
) -> io::Result<Option<SuccessfulTopbaseResult<C>>> {
    match hashing_mode {
        BlobHashingMode::Full => simplest_topbase_inner::<RawBlobSummary, C, NopCB>(
            a_committish, b_committish, false, None),
        BlobHashingMode::WithoutPath => simplest_topbase_inner::<RawBlobSummaryWithoutPath, C, NopCB>(
            a_committish, b_committish, false, None),
        BlobHashingMode::EndState => simplest_topbase_inner::<RawBlobSummaryEndState, C, NopCB>(
            a_committish, b_committish, false, None),
        BlobHashingMode::EndStateWithoutPath => simplest_topbase_inner::<RawBlobSummaryEndStateWithoutPath, C, NopCB>(
            a_committish, b_committish, false, None),
    }
}

pub fn rewind_topbase<
    C: From<CommitWithBlobs>,
    B: FnMut(&mut RawBlobSummary, &str) -> bool,
>(
    a_committish: &str, b_committish: &str,
    hashing_mode: BlobHashingMode,
    should_use_blob_cb: Option<B>,
) -> io::Result<Option<SuccessfulTopbaseResult<C>>> {
    match hashing_mode {
        BlobHashingMode::Full => simplest_topbase_inner::<RawBlobSummary, C, B>(
            a_committish, b_committish, true, should_use_blob_cb),
        BlobHashingMode::WithoutPath => simplest_topbase_inner::<RawBlobSummaryWithoutPath, C, B>(
            a_committish, b_committish, true, should_use_blob_cb),
        BlobHashingMode::EndState => simplest_topbase_inner::<RawBlobSummaryEndState, C, B>(
            a_committish, b_committish, true, should_use_blob_cb),
        BlobHashingMode::EndStateWithoutPath => simplest_topbase_inner::<RawBlobSummaryEndStateWithoutPath, C, B>(
            a_committish, b_committish, true, should_use_blob_cb),
    }
}


// TODO:
// - rewrite the blob set to be a blob map. need a way to find which
// commit a blob points to...

#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;

    fn default_use_blob(_b: &mut RawBlobSummary, _s: &str) -> bool {
        true
    }

    #[test]
    fn loader_doesnt_make_more_groups_than_commit_count() {
        // we pass a very high value of N, if we call
        // load_next() several times, there should still only be 1 group
        // TODO: if this repo ever gets more than 1000000 commits, update this:
        let mut loader = BranchIterativeCommitLoader::<RawBlobSummary>::new(1000000, "HEAD");
        loader.load_next(&mut default_use_blob, |_| false).unwrap();
        assert_eq!(loader.groups.len(), 1);
        let num_commits = loader.groups[0].len();
        loader.load_next(&mut default_use_blob, |_| false).unwrap();
        loader.load_next(&mut default_use_blob, |_| false).unwrap();
        loader.load_next(&mut default_use_blob, |_| false).unwrap();
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
        loader.load_next(&mut default_use_blob, |_| false).unwrap();
        assert_eq!(loader.groups.len(), 1);
        assert_eq!(loader.groups[0].len(), 1);
        let first_commit = loader.groups[0][0].0.commit.id.hash.clone();
        loader.load_next(&mut default_use_blob, |_| false).unwrap();
        assert_eq!(loader.groups.len(), 2);
        assert_eq!(loader.groups[0].len(), 1);
        assert_eq!(loader.groups[1].len(), 1);
        let next_commit = loader.groups[1][0].0.commit.id.hash.clone();
        assert!(first_commit != next_commit);
    }
}
