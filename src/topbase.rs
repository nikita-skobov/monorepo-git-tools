use std::{io, collections::HashSet, process::Stdio};
use io::{BufReader, BufRead};

use super::ioerr;
use super::git_helpers3;
use super::git_helpers3::Commit;
use super::git_helpers3::Oid;
use super::exec_helpers;
use super::check::topbase_check_alg;
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
pub enum BlobMode {
    Add,
    Modify,
    Delete,
    // TODO: handle Rename (R100)
}

#[derive(Debug)]
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
    let blob_mode = match *diff_type {
        "A" => BlobMode::Add,
        "D" => BlobMode::Delete,
        _ => BlobMode::Modify,
        // TODO: Handle renames.. the path component is actually <SRC>\t<DEST>
    };
    // if its a delete blob, we use the previous blob id
    // otherwise we use the current
    let blob_id = if let BlobMode::Delete = blob_mode {
        blob_prev
    } else {
        blob_next
    };
    let blob = Blob {
        mode: blob_mode,
        id: blob_id.to_string(),
        path: blob_path,
    };

    Ok(blob)
}

/// read from a buf read interface of a list of lines that contain the git log
/// output corresponding to the specifically --raw --pretty=oneline format.
/// Parse line by line and return a blob set of all of the blobs in this output
/// as well as a list of all commits.
/// optionally pass in a callback to modify/inspect the blobs/commits before
/// they are inserted into the output. This callback function is optional. If you want
/// to use the default behavior, you can pass: `|_, _| true`
pub fn generate_commit_list_and_blob_set_from_lines<T: BufRead>(
    line_reader: &mut T,
    should_add: impl FnMut(&mut Commit, &mut Vec<Blob>) -> bool,
) -> io::Result<AllCommitsAndBlobs> {
    let mut should_add = should_add;
    let mut out = AllCommitsAndBlobs::default();
    let mut last_commit = Commit::new("", "".into(), true);
    let mut last_blobs = vec![];
    let mut add_last_commit = false;

    for line in line_reader.lines() {
        let line = line?;
        if ! line.starts_with(':') {
            // parsing a commit line
            if add_last_commit {
                if should_add(&mut last_commit, &mut last_blobs) {
                    out.commits.push((last_commit, last_blobs));
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
    }

    // after iteration have to add the last one:
    if should_add(&mut last_commit, &mut last_blobs) {
        out.commits.push((last_commit, last_blobs));
    }

    Ok(out)
}

/// same as `generate_commit_list_and_blob_set` but you can specify
/// a callback to evaluate/modify the commits/blobs before they are
/// added to the output
pub fn generate_commit_list_and_blob_set_with_callback<T>(
    committish: &str,
    callback: Option<T>
) -> io::Result<AllCommitsAndBlobs>
    where T: FnMut(&mut Commit, &mut Vec<Blob>) -> bool
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
        None => generate_commit_list_and_blob_set_from_lines(&mut stdout_read, |_, _| true),
    };
    // let stdout_lines = stdout_read.lines();

    let exit = child.wait()?;
    // eprintln!("{:?}", exit);

    output
}

pub const NOP_CB: Option<fn (&mut Commit, &mut Vec<Blob>) -> bool> = None;

/// specify a committish of what branch/commit youd like to pass to
/// `git log --raw --pretty=oneline <committish>`
pub fn generate_commit_list_and_blob_set(
    committish: &str
) -> io::Result<AllCommitsAndBlobs> {
    generate_commit_list_and_blob_set_with_callback(committish, NOP_CB)
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
pub fn find_a_b_difference(
    a_committish: &str, b_committish: &str,
    // TODO: different modes of traversal. default
    // should be Topbase, but maybe also include Fullbase which will
    // look for every possible difference instead of stopping at the first one?
    // TODO: add stop at X commit for both A and B branch.
) -> io::Result<()> {
    let fully_loaded_b = generate_commit_list_and_blob_set(b_committish)?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;

    #[test]
    fn commit_list_properly_detects_merge_commits() {
        let log_output = "somehash commit message here\n01010101010110 another commit message here";
        let mut cursor = Cursor::new(log_output.as_bytes());
        let all_things = generate_commit_list_and_blob_set_from_lines(&mut cursor, |_, _| true).unwrap();
        assert_eq!(all_things.commits.len(), 2);
        assert!(all_things.commits[0].0.is_merge);
        assert!(all_things.commits[1].0.is_merge);
    }

    #[test]
    fn commit_list_properly_parses_blobs() {
        let log_output = "hash1 msg1\n:100644 100644 xyz abc M file1.txt\n:100644 00000 123 000 D file2.txt";
        let mut cursor = Cursor::new(log_output.as_bytes());
        let all_things = generate_commit_list_and_blob_set_from_lines(&mut cursor, |_, _| true).unwrap();
        assert_eq!(all_things.commits.len(), 1);
        assert!(!all_things.commits[0].0.is_merge);
        let blobs = &all_things.commits[0].1;
        assert_eq!(blobs.len(), 2);
        assert_eq!(blobs[0].mode, BlobMode::Modify);
        assert_eq!(blobs[0].id, "abc");
        assert_eq!(blobs[1].mode, BlobMode::Delete);
        assert_eq!(blobs[1].id, "123");
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
