use std::{io, collections::HashSet, process::Stdio, str::FromStr, iter::FromIterator, hash::Hash};
use io::{BufReader, BufRead, Lines};

use super::ioerr;
use super::git_helpers3;
use super::git_helpers3::Commit;
use super::git_helpers3::Oid;
use super::git_helpers3::{RawBlobSummaryWithoutPath, RawBlobSummary};
use super::exec_helpers;
use super::die;
use super::cli::MgtCommandTopbase;

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

    let current_commits_not_in_upstream = simplest_topbase(&current_branch, &upstream_branch, true)
        .map_err(|e| e.to_string())?;
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

// refactor 'topbase' and also add other algorithms
// (mainly 'fullbase' which would find all commits that differ
// between two branches, even if theres some stuff in the middle.)
// get rid of the "one branch gets all blobs, the other gets all commits, and then we search
// in order from top to bottom of the one where we found the commits"...
// its unnecessary and creates complexity about "which one should be top/bottom?"
// It should be possible to only use git-rev-list to get all commits, and all blobs of those commits
// from both branches.
// also: might want to look into reading two streams of git-rev-list from the two branches
// instead of allocating all of that memory at once...
// alsO: might want to look into commit limiting. could be useful for something where
// we 'know' that we already are ahead of remote at point X, so no point
// in looking before X. This would be useful for massive repos.
// the command for that would be:
// git --no-pager log --raw --pretty=oneline <branch-name>
// optionally add a "-m" if you DO WANT merge commits to show
// a diff format with blobs/trees. this would be useful if you want
// to explicitly find merge commits when finding a fork point, which is
// NOT what we do by default anyway (ie: by default dont pass the -m)

#[derive(Debug, PartialOrd, PartialEq)]
pub enum ShouldAddMode {
    Add,
    DontAdd,
    AddAndExit,
    Exit,
}

#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum BlobMode {
    Add,
    Modify,
    Delete,
    Rename,
}

#[derive(Debug, Clone)]
pub struct Blob {
    pub mode: BlobMode,
    pub id: String,
    pub path: String,
}

// TODO: Do we want the blob_set to be a blob_map
// and point to the commit that its part of?
// but a blob can belong to many commits...
// also do we want to know the blobs that pertain to an individual commit?
#[derive(Default)]
pub struct AllCommitsAndBlobs {
    pub blob_set: HashSet<String>,
    pub commits: Vec<(Commit, Vec<Blob>)>,
}

impl AllCommitsAndBlobs {
    /// returns true if this blob_set contains every single blob
    /// in the provided list of blobs
    pub fn contains_all_blobs(&self, blobs: &[Blob]) -> bool {
        let mut contains_all = true;
        for blob in blobs {
            if ! self.blob_set.contains(&blob.id) {
                contains_all = false;
                break;
            }
        }
        contains_all
    }
}

pub fn parse_blob_from_line(line: &str) -> io::Result<Blob> {
    let items = line.split_whitespace().collect::<Vec<&str>>();
    // there are technically 6 items from this output:
    // the last item (items[5]) is a path to the file that this blob
    // is for (and the array could have more than 6 if file names
    // have spaces in them)
    let (
        // TODO: do we care about the modes?
        _mode_prev, _mode_next,
        blob_prev, blob_next,
        diff_type
    ) = (
        items.get(0).ok_or_else(|| ioerr!("Failed to parse git blob line: {}", line))?,
        items.get(1).ok_or_else(|| ioerr!("Failed to parse git blob line: {}", line))?,
        items.get(2).ok_or_else(|| ioerr!("Failed to parse git blob line: {}", line))?,
        items.get(3).ok_or_else(|| ioerr!("Failed to parse git blob line: {}", line))?,
        items.get(4).ok_or_else(|| ioerr!("Failed to parse git blob line: {}", line))?
    );

    // the path of this blob starts at index 5, but we combine the rest
    // in case there are spaces
    let blob_path = items.get(5..)
        .ok_or_else(|| ioerr!("Failed to parse git blob line: {}", line))?.join(" ");
    let mut blob_mode = match *diff_type {
        "A" => BlobMode::Add,
        "D" => BlobMode::Delete,
        _ => BlobMode::Modify,
        // TODO: Handle renames.. the path component is actually <SRC>\t<DEST>
    };
    // ive seen it as R, and also as R100, so I dont
    // want to just rely on pattern matching...
    // if it starts with an R, we consider this a rename:
    if diff_type.starts_with("R") {
        blob_mode = BlobMode::Rename;
    }
    // if its a delete blob, we use the previous blob id
    // otherwise we use the current
    let blob_id = if let BlobMode::Delete = blob_mode {
        format!("D_{}", blob_prev)
    } else if let BlobMode::Rename = blob_mode {
        format!("R_{}", blob_next)
    } else {
        blob_next.to_string()
    };
    let blob = Blob {
        mode: blob_mode,
        path: blob_path,
        id: blob_id,
    };

    Ok(blob)
}

/// read from a buf read interface of a list of lines that contain the git log
/// output corresponding to the specifically --raw --pretty=oneline format.
/// Parse line by line and return a blob set of all of the blobs in this output
/// as well as a list of all commits.
/// optionally pass in a callback to modify/inspect the blobs/commits before
/// they are inserted into the output. This callback function is optional. If you want
/// to use the default behavior, you can pass: `|_, _| ShouldAddMode::Add`
/// The response is a tuple that contains (kill the child process of this command, AllCommitsAndBlobs)
/// If `should_kill_child` is true, you should/can kill the child process that we are reading from
/// If you are running this command from your own in-memory stream/buffer, you can ignore that field.
pub fn generate_commit_list_and_blob_set_from_lines<T: BufRead>(
    line_reader: &mut T,
    should_add: impl FnMut(&mut Commit, &mut Vec<Blob>) -> ShouldAddMode,
) -> io::Result<(bool, AllCommitsAndBlobs)> {
    let mut should_add_cb = should_add;
    let mut out = AllCommitsAndBlobs::default();
    let mut last_commit = Commit::new("", "".into(), true);
    let mut last_blobs = vec![];
    let mut add_last_commit = false;

    let mut buf = vec![];
    while let Ok(bytes_read) = line_reader.read_until(b'\n', &mut buf) {
        if bytes_read == 0 {
            break;
        }
        let line = String::from_utf8_lossy(&buf);
        let line = &line[..];
        let line_len = line.len();
        let line = if line.ends_with('\n') {
            &line[0..line_len - 1]
        } else { line };
        if ! line.starts_with(':') {
            // parsing a commit line
            if add_last_commit {
                let (should_add, should_exit) = match should_add_cb(&mut last_commit, &mut last_blobs) {
                    ShouldAddMode::Add => (true, false),
                    ShouldAddMode::AddAndExit => (true, true),
                    ShouldAddMode::DontAdd => (false, false),
                    ShouldAddMode::Exit => (false, true),
                };
                if should_add {
                    out.commits.push((last_commit, last_blobs));
                }
                if should_exit {
                    return Ok((true, out));
                }
                last_blobs = vec![];
                last_commit = Commit::new("", "".into(), true);
            }

            let first_space_index = line.find(' ').ok_or(ioerr!("Failed to read line of git log output:\n{}", line))?;
            let hash = &line[0..first_space_index];
            let summary = &line[(first_space_index+1)..];
            last_commit.id = Oid { hash: hash.to_string() };
            last_commit.summary = summary.to_string();
            add_last_commit = true;
        } else {
            // parsing a blob line

            // if we see a blob, then by definition that means its not
            // a merge commit because in our git log format we dont pass the '-m' flag
            // TODO: what happens if we do pass that?
            last_commit.is_merge = false;
            let blob = parse_blob_from_line(&line)?;
            out.blob_set.insert(blob.id.clone());
            last_blobs.push(blob);
        }
        buf.clear();
    }

    let should_add = match should_add_cb(&mut last_commit, &mut last_blobs) {
        ShouldAddMode::Add => true,
        ShouldAddMode::AddAndExit => true,
        _ => false,
    };

    // after iteration have to add the last one:
    if should_add {
        out.commits.push((last_commit, last_blobs));
    }
    // no point in checking if should_exit because we are exiting here anyway

    Ok((false, out))
}

/// same as `generate_commit_list_and_blob_set` but you can specify
/// a callback to evaluate/modify the commits/blobs before they are
/// added to the output
pub fn generate_commit_list_and_blob_set_with_callback<T>(
    committish: &str,
    callback: Option<T>
) -> io::Result<AllCommitsAndBlobs>
    where T: FnMut(&mut Commit, &mut Vec<Blob>) -> ShouldAddMode
{
    // TODO: allow us to specify a stopping commit
    // TODO: add the '-m' flag if we want to see merge commits with a full blob diff
    let exec_args = [
        "git", "--no-pager", "log", "--no-color", "--raw", "--pretty=oneline", committish
    ];

    let mut child = exec_helpers::spawn_with_env_ex(
        &exec_args,
        &[], &[],
        None, None, Some(Stdio::piped()),
    )?;

    let stdout = child.stdout.as_mut()
        .ok_or(ioerr!("Failed to get child stdout for reading git log of {}", committish))?;
    let mut stdout_read = BufReader::new(stdout);
    let output = match callback {
        Some(cb) => generate_commit_list_and_blob_set_from_lines(&mut stdout_read, cb),
        None => generate_commit_list_and_blob_set_from_lines(&mut stdout_read, |_, _| ShouldAddMode::Add),
    };

    let (should_kill_child, output) = match output {
        Ok(o) => {
            (o.0, Ok(o.1))
        }
        Err(e) => {
            (true, Err(e))
        }
    };

    if should_kill_child {
        // I think if we failed to kill child we dont care?
        // I think if this errors, it means the child already was killed
        // so we still want to exit...
        let _ = child.kill();
    } else {
        // only return this child.wait() error if
        // our output response is ok. if our output is an error,
        // then we would rather return that error instead of an error
        // that came from calling child.wait()
        let child_wait_res = child.wait();
        if output.is_ok() {
            let _ = child_wait_res?;
        }
    }

    output
}

// thank you: https://users.rust-lang.org/t/option-fn-and-type-inference-for-none-case/51611/5
pub const NOP_CB: Option<fn (&mut Commit, &mut Vec<Blob>) -> ShouldAddMode> = None;

/// specify a committish of what branch/commit youd like to pass to
/// `git log --raw --pretty=oneline <committish>`
pub fn generate_commit_list_and_blob_set(
    committish: &str
) -> io::Result<AllCommitsAndBlobs> {
    generate_commit_list_and_blob_set_with_callback(committish, NOP_CB)
}

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

/// a group of consecutive commits.
/// if `is_shared` is true then this consecutive group of commits
/// exists in another branch (although possibly by different commit hashes
/// but can be looked up via blob ids). Throughout the
/// documentation we use the terms exclusive and shared.
/// `is_shared` is the opposite of exclusive
#[derive(Debug, Default, Clone)]
pub struct ConsecutiveCommitGroup {
    pub commits: Vec<(Commit, Vec<Blob>)>,
    pub is_shared: bool,
}

/// keep track of a list of consecutive commits. When calling advance, we will
/// add to the consecutive group, or make a new group if we found a non-consecutive
/// commit. Call unfold() when done, and get back a Vec of ConsecutiveCommitGroups
#[derive(Debug, Default)]
pub struct ConsecutiveCommitGroups {
    pub groups: Vec<ConsecutiveCommitGroup>,
    pub current_group: Option<ConsecutiveCommitGroup>,
    pub last_commit_was_exclusive: bool,
}

impl ConsecutiveCommitGroups {
    pub fn make_new_commit_group(
        &self,
        this_commit_is_exclusive: bool,
        this_commit: (&mut Commit, &&mut Vec<Blob>),
    ) -> ConsecutiveCommitGroup {
        let (commit, blobs) = this_commit;
        let commits = vec![(commit.clone(), (*blobs).clone())];
        // we start a commit group:
        let new_commit_group = ConsecutiveCommitGroup {
            commits,
            is_shared: ! this_commit_is_exclusive,
        };
        new_commit_group
    }

    pub fn advance_same(
        &mut self,
        this_commit_is_exclusive: bool,
        this_commit: (&mut Commit, &&mut Vec<Blob>),
        should_collect_shared: bool,
    ) {
        if let Some(ref mut current_group) = self.current_group {
            // append a commit to current group
            let (commit, blobs) = this_commit;
            current_group.commits.push((commit.clone(), (*blobs).clone()));
        } else {
            // DO NOT create a new group if we are only interested
            // in exclusive groups, and this commit is shared:
            if ! should_collect_shared && ! this_commit_is_exclusive {
                return;
            }
            self.current_group = Some(self.make_new_commit_group(this_commit_is_exclusive, this_commit));
        }
    }

    pub fn advance_different(
        &mut self,
        this_commit_is_exclusive: bool,
        this_commit: (&mut Commit, &&mut Vec<Blob>),
        should_collect_shared: bool,
    ) {
        // we reached the end of the last group,
        // ad it to the list, and reset us to None
        if let Some(ref mut current_group) = self.current_group {
            self.groups.push(current_group.clone());
            self.current_group = None;
        }

        // DO NOT create a new group if we are only interested
        // in exclusive groups, and this commit is shared:
        if ! should_collect_shared && ! this_commit_is_exclusive {
            return;
        }

        // if we do want to collect shared groups, or if
        // this commit is exclusive then go ahead and make this new group:
        // and also create a new group:
        self.current_group = Some(self.make_new_commit_group(this_commit_is_exclusive, this_commit));
    }

    pub fn advance(
        &mut self,
        this_commit_is_exclusive: bool,
        this_commit: (&mut Commit, &&mut Vec<Blob>),
        should_collect_shared: bool,
    ) {
        match (self.last_commit_was_exclusive, this_commit_is_exclusive) {
            // the last commit was exclusive, and therefore not shared.
            // this commit is also exclusive, therefore we just append the
            // current group, or start a new one that is not shared
            (true, true) |
            // both the last commit and this commit are shared. just add it to the
            // current group, or make a new one that is shared.
            // NOTE: the case here is the same because we are just appending
            // to an existing group or making a new one. we are not closing off
            // a group.
            (false, false) => {
                self.advance_same(
                    this_commit_is_exclusive, this_commit, should_collect_shared);
            },

            // the last commit was shared, this commit is exclusive. therefore
            // we finish the last group and add it to the groups list,
            // and we start a new group that is exclusive
            (false, true) |
            // the last commit was exclusive, but this commit is shared.
            // finish off the last group, add it to the list, and start a new group
            // that is shared
            (true, false) => {
                self.advance_different(
                    this_commit_is_exclusive, this_commit, should_collect_shared);
            },
        }

        self.last_commit_was_exclusive = this_commit_is_exclusive;
    }

    /// get back a vec of consecutive commit groups
    pub fn unfold(&mut self) -> Result<Vec<ConsecutiveCommitGroup>, String> {
        let mut out = vec![];

        if let Some(dangling_group) = &self.current_group {
            self.groups.push(dangling_group.clone());
            self.current_group = None;
        }

        for commit_group in self.groups.drain(..) {
            let out_group = ConsecutiveCommitGroup {
                commits: commit_group.commits,
                is_shared: commit_group.is_shared,
            };
            out.push(out_group);
        }

        Ok(out)
    }
}

/// a callback helper for making your own parser of
/// `generate_commit_list_and_blob_set_with_callback`
/// this function should be called from within your closure
/// that is passed to the generate commit list.
/// Note that it depends on several mutable variables, so youd need
/// to create those beforehand. See the `find_a_b_difference`
/// function for an example of how this callback can be used.
/// Example:
/// ```
/// let cb = |commit: &mut Commit, blobs: &mut Vec<Blob>| -> ShouldAddMode {
///     should_add_commit_callback_helper(
///         commit, blobs,
///         &fully_loaded_b, &mut a_has_but_not_in_b,
///         &mut stop_search_b_at_blobs,
///         is_fullbase, should_rewind, is_regular_topbase,
///     )
/// };
/// generate_commit_list_and_blob_set_with_callback(a_committish, Some(cb))?;
/// ```
pub fn should_add_commit_callback_helper(
    commit: &mut Commit, blobs: &mut Vec<Blob>,
    fully_loaded_b: &AllCommitsAndBlobs,
    a_output: &mut ConsecutiveCommitGroups,
    stop_search_b_at_blobs: &mut Option<Vec<Blob>>,
    is_fullbase: bool,
    is_rewind: bool,
    should_collect_shared: bool,
) -> ShouldAddMode {
    // TODO: what if we want merge commits?
    // by default skip merge commits:
    if commit.is_merge {
        return ShouldAddMode::DontAdd;
    }
    // TODO: is it sufficient to say "this commit in A exists in B because
    // all of the blob hashes of the A commit exists **somewhere** in B?"
    // it is certainly computationally efficient, but is it correct?
    // consider example where A has 3 blobs, and those 3 blobs exist
    // in *different* commits in B. Should that still count as the A commit
    // existing? I think maybe not... I think that the most correct
    // appraoch would be to check through all of B's commits to see if there is
    // a commit that contains all of the blobs of the current A's commit. However that
    // is much more computationally expensive, and Im leaning towards its unlikely
    // that a scenario like this would occur in a real code base.. but who knows...
    let should_add_to_a = ! fully_loaded_b.contains_all_blobs(blobs);
    // if should_add_to_a {
    //     println!("FOUND COMMIT IN A THAT IS NOT IN B:\n{} {}\n", commit.id.short(), commit.summary);
    // }

    let take_commit = (commit, &blobs);
    a_output.advance(should_add_to_a, take_commit, should_collect_shared);

    // fullbase mode: always add
    if is_fullbase {
        return ShouldAddMode::Add;
    }

    // in topbase mode:
    // if we are not rewinding, we can just
    // exit if we find a commit that is in both A and B
    if ! is_rewind {
        // dont be confused by the semantics here of "should_add_to_a"
        // we are 'adding' this commit locally above in the
        // `a_has_but_not_in_b.advance(should_add_to_a);`
        // but by telling our caller to "DontAdd", we are basically saying
        // dont allocate memory for all of these commits, because
        // we will do that ourselves instead.
        return if ! should_add_to_a {
            ShouldAddMode::Exit
        } else {
            ShouldAddMode::DontAdd
        };
    }

    // otherwise we are in topbase:rewind mode:
    // if we reached a commit that is in both A, and B
    // we can add it and exit and then rewind from this point
    if ! should_add_to_a {
        // this is the last commit in rewind mode.
        // make sure to save its list of blobs so that when we
        // are traversing B, we know when to stop:
        *stop_search_b_at_blobs = Some(blobs.clone());
        ShouldAddMode::AddAndExit
    } else {
        ShouldAddMode::Add
    }
}

/// given two branch/committishes A, and B
/// find the differences between them. NOTE THAT
/// B is the 'bottom' branch which implies its entire log is loaded into memory
/// first, whereas A's branch is traversed one entry at a time, and not loaded into memory.
/// There are tradeoffs to which branch should be A, and which should be B. A common example:
/// - If you are fairly confident that branch Y is ahead of branch X, you should make 
///   call `find_a_b_difference(Y, X)` because X will get entirely loaded, and then we only
///   traverse the tip of Y until we find their shared fork point (using topbase mode)
/// - If you know that branch X is massive, but branch Y is not that big, then you have a
///   tradeoff: by making X the B branch, it will get loaded entirely, which is potentially
///   a lot of memory, but then searching through the Y branch is really fast because
///   now we have a fully built map to the X branch. Conversely, we can make Y the B branch
///   and then we traverse the X branch one commit at a time, so we use less memory, but
///   our traversal would be quite a bit slower.
/// Also, you have a choice of traversal mode. See the documentation for `ABTraversalMode`
/// You can pass in None, if you wish to use the default traversal mode.
/// returns a tuple of (A ConsecutiveCommitGroup, B ConsecutiveCommitGroup)
/// where the consecutive commit group for A contains commits that A has, but B does not,
/// and the consecutive commit group for B contains commits that B has, but A does not.
/// again: the "Y has, but X does not" is specific to the traversal mode. If you want to find
/// ALL of the possible different commits, use Fullbase as your traversal mode.
/// Also pass a `should_collect_shared` flag. If true, then the returned consecutive groups
/// can be either shared or exclusive. A shared group means every commit in that group
/// exists somewhere in the other branch. An exclusive group means this range of commits is
/// unique to this branch. For performing a topbase rebase, you would want
/// to pass `should_collect_shared = false` because you only care about
/// the top-most exclusive commit group. But for diff-log, you would pass
/// `should_collect_shared = true` because you want to see a
/// list of shared/exclusive commits
pub fn find_a_b_difference<T: Into<Option<ABTraversalMode>>>(
    a_committish: &str, b_committish: &str,
    traversal_mode: T,
    should_collect_shared: bool,
    // TODO: add stop at X commit for both A and B branch.
) -> io::Result<(Vec<ConsecutiveCommitGroup>, Vec<ConsecutiveCommitGroup>)> {
    let traversal_mode = traversal_mode.into().unwrap_or(ABTraversalMode::default());
    let (should_rewind, is_fullbase) = match traversal_mode {
        ABTraversalMode::Topbase => (false, false),
        ABTraversalMode::TopbaseRewind => (true, false),
        ABTraversalMode::Fullbase => (false, true),
    };
    // let is_regular_topbase = !should_rewind && !is_fullbase;
    let mut fully_loaded_b = generate_commit_list_and_blob_set(b_committish)?;

    let mut stop_search_b_at_blobs = None;
    let mut a_has_but_not_in_b = ConsecutiveCommitGroups::default();
    let cb = |commit: &mut Commit, blobs: &mut Vec<Blob>| -> ShouldAddMode {
        should_add_commit_callback_helper(
            commit, blobs,
            &fully_loaded_b, &mut a_has_but_not_in_b,
            &mut stop_search_b_at_blobs,
            is_fullbase, should_rewind,
            should_collect_shared,
        )
    };
    let a_commits = generate_commit_list_and_blob_set_with_callback(a_committish, Some(cb))?;

    // in regular topbase we do not need to search through B's commits
    let mut b_has_but_not_in_a = ConsecutiveCommitGroups::default();
    if is_fullbase || should_rewind {
        for (ref mut commit, ref mut blobs) in &mut fully_loaded_b.commits {
            if let Some(ref stop_at_blobs) = stop_search_b_at_blobs {
                // if we found the commit where we previously stopped searching A,
                // then this is where we also stop searching B. this is only valid for
                // rewind mode
                if all_blobs_exist(stop_at_blobs, &blobs) {
                    // if we wish to collect shared commits,
                    // we must advance one last time before we break:
                    if should_collect_shared {
                        // we say false here because we know this is
                        // not an exclusive commit
                        let this_commit = (commit, &blobs);
                        b_has_but_not_in_a.advance(false, this_commit, should_collect_shared);
                    }
                    break;
                }
            }
            let should_add_to_b = ! a_commits.contains_all_blobs(&blobs);
            // if should_add_to_b {
            //     println!("FOUND COMMIT IN B THAT IS NOT IN A:\n{} {}\n", commit.id.short(), commit.summary);
            // }
            let this_commit = (commit, &blobs);
            b_has_but_not_in_a.advance(should_add_to_b, this_commit, should_collect_shared);
        }
    }

    let consecutive_groups_in_a = a_has_but_not_in_b.unfold()
        .map_err(|e| ioerr!("{}", e))?;
    let consecutive_groups_in_b = b_has_but_not_in_a.unfold()
        .map_err(|e| ioerr!("{}", e))?;
    

    Ok((consecutive_groups_in_a, consecutive_groups_in_b))
}

/// returns true if all of A's blobs exist in B
pub fn all_blobs_exist(a: &[Blob], b: &[Blob]) -> bool {
    let mut contains_all = true;
    for blob in a {
        let b_contains_a_blob = b.iter().any(|x| x.id == blob.id);
        if ! b_contains_a_blob {
            contains_all = false;
            break;
        }
    }
    return contains_all;
}

/// this is called by `find_a_b_difference2` if you
/// passed a Some(n) for the traverse n at a time.
pub fn find_a_b_difference2_iterative_traversal(
    a_committish: &str, b_committish: &str,
    traverse_n: usize,
) {
    // let mut b_commits = vec![];

    // first we load N of the B branches commits:
}

pub fn find_a_b_difference2(
    a_committish: &str, b_committish: &str,
    traverse_n_at_a_time: Option<usize>,
    // TODO: add traversal mode...
) {

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
/// pass true if you want to consider paths when comparing blobs
/// or false if you want to make this a bit faster (but possibly slightly less correct)
pub fn simplest_topbase(
    a_committish: &str, b_committish: &str,
    consider_paths: bool,
) -> io::Result<Option<SuccessfulTopbaseResult>> {
    if consider_paths {
        simplest_topbase_inner::<RawBlobSummary>(a_committish, b_committish)
    } else {
        simplest_topbase_inner::<RawBlobSummaryWithoutPath>(a_committish, b_committish)
    }
}


// TODO:
// - rewrite the find_ab_diff function to:
// only return a single output collection. it should
// have references to the different groups it finds as it
// traverses
// - rewrite the blob set to be a blob map. need a way to find which
// commit a blob points to...
// - instead of hashing by a single blob and checking "does this blob
// exist somewhere in the other branch?" we should
// create an entire set of blobs, and then check if the other branch
// has a commit such that our set of blobs is a subset of that
// commit... (or vice versa?)
// - refactor the command stdout processing to try to
// just read 1000 commits from the B branch at a time without
// loading it entirely. Then load 1000 commits from the A branch and check
// through the 1000 we loaded from B. its possible we find our top-most
// fork point in those first 1000 and then have no need to load the next.
// but if we dont, then we load another 1000 from B and check all of the ones in A
// against the ones we just loaded (BUT NOT CHECKING THE ONES FROM THE BEGINNING
// BECAUSE WE KNOW ITS NOT IN THERE). Then if we again dont find anything, we
// now load 1000 from A again, and this time these new 1000 from A need to be checked
// against everything in B. we keep doing this until we have either found
// the fork point we are looking for, or until we load the entirety of the branches.
// i think this will be the optimal strategy for 99% of cases because chances are
// we will find the fork point in the first thousand...
// if doing this, you can do:
// git log --raw --pretty=oneline <last-seen-sha> -n 1000
// NOTE that you can do <last-seen-sha>^1 if you want
// to get everything AFTER the last seen sha.. but theres a risk
// that if your last seen sha is the first commit, then if you run that
// you will get an error because there is no commit
// past that..

#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;

    #[test]
    fn commit_list_properly_detects_merge_commits() {
        let log_output = "somehash commit message here\n01010101010110 another commit message here";
        let mut cursor = Cursor::new(log_output.as_bytes());
        let (_, all_things) = generate_commit_list_and_blob_set_from_lines(&mut cursor, |_, _| ShouldAddMode::Add).unwrap();
        assert_eq!(all_things.commits.len(), 2);
        assert!(all_things.commits[0].0.is_merge);
        assert!(all_things.commits[1].0.is_merge);
    }

    #[test]
    fn commit_list_properly_parses_blobs() {
        let log_output = "hash1 msg1\n:100644 100644 xyz abc M file1.txt\n:100644 00000 123 000 D file2.txt";
        let mut cursor = Cursor::new(log_output.as_bytes());
        let (_, all_things) = generate_commit_list_and_blob_set_from_lines(&mut cursor, |_, _| ShouldAddMode::Add).unwrap();
        assert_eq!(all_things.commits.len(), 1);
        assert!(!all_things.commits[0].0.is_merge);
        let blobs = &all_things.commits[0].1;
        assert_eq!(blobs.len(), 2);
        assert_eq!(blobs[0].mode, BlobMode::Modify);
        assert_eq!(blobs[0].id, "abc");
        assert_eq!(blobs[1].mode, BlobMode::Delete);
        // a delete blob should be prefixed by D_ to
        // differentiate from when it was added/modified
        assert_eq!(blobs[1].id, "D_123");
    }

    // not exactly a unit test, but simple enough to implement:
    // basically just want to check if it runs successfully on a real
    // repo. this assumes this command is ran from a valid repo
    #[test]
    fn commit_list_properly_runs_from_head() {
        let all_things = generate_commit_list_and_blob_set("HEAD").unwrap();
        for (commit, blobs) in all_things.commits {
            println!("{} {}\n{:#?}\n", commit.id.short(), commit.summary, blobs);
        }
    }
}
