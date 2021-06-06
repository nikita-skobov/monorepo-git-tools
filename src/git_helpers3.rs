/// For the v3 version I rewrote the git_helpers module to interface
/// with git via the CLI instead of libgit2

use super::exec_helpers;
use std::{io::{self, BufReader}, io::BufRead, process::Stdio, str::FromStr};
use crate::{ioerre, ioerr};

#[derive(Debug, Clone)]
pub struct Oid {
    pub hash: String,
}
impl Oid {
    /// it is assumed that short() will not be called
    /// on an empty oid
    pub fn short(&self) -> &str {
        let substr = self.hash.get(0..7);
        substr.unwrap()
    }
    pub fn long(&self) -> &String {
        &self.hash
    }
}
#[derive(Debug, Clone)]
pub struct Commit {
    pub id: Oid,
    pub summary: String,
    pub is_merge: bool,
}

impl Commit {
    pub fn new(hash: &str, summary: String, is_merge: bool) -> Commit {
        let oid = Oid { hash: hash.to_string() };
        Commit { id: oid, summary, is_merge }
    }
}

#[derive(Debug, Clone)]
pub struct CommitWithBlobs {
    pub commit: Commit,
    pub blobs: Vec<RawBlobSummary>,
}

/// see: https://www.git-scm.com/docs/git-diff
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum DiffStatus {
    Added,
    Copied,
    Deleted,
    Modified,
    Renamed,
    TypeChanged,
    Unmerged,
    Unknown,
}

impl FromStr for DiffStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ok = match &s[0..1] {
            "A" => DiffStatus::Added,
            "C" => DiffStatus::Copied,
            "D" => DiffStatus::Deleted,
            "M" => DiffStatus::Modified,
            "R" => DiffStatus::Renamed,
            "T" => DiffStatus::TypeChanged,
            "U" => DiffStatus::Unmerged,
            "X" => DiffStatus::Unknown,
            _ => return Err(format!("Failed to get diff status from {}", s)),
        };
        Ok(ok)
    }
}

/// see: https://stackoverflow.com/questions/737673/how-to-read-the-mode-field-of-git-ls-trees-output
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum FileMode {
    Empty,
    Directory,
    RegularNonEx,
    RegularNonExGroupWrite,
    RegularEx,
    SymbolicLink,
    GitLink,
    // Im adding this one just in case... im not sure
    // if the above are all of the valid file modes,
    // so in case we cant parse it, we will say its unknown...
    Unknown,
}

impl FromStr for FileMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ok = match s {
            "000000" => FileMode::Empty,
            "040000" => FileMode::Directory,
            "100644" => FileMode::RegularNonEx,
            "100664" => FileMode::RegularNonExGroupWrite,
            "100755" => FileMode::RegularEx,
            "120000" => FileMode::SymbolicLink,
            "160000" => FileMode::GitLink,
            _ => FileMode::Unknown,
        };
        Ok(ok)
    }
}

/// there are 8 possible values for the DiffStatus enum,
/// as well as the FileMode. that means to represent a single
/// item (where the item includes the source filemode, dest filemode
/// and the diff status) we only need 8 * 8 * 8 = 64 possible states, so
/// we can store that in 9 bits, or otherwise: a u16
/// packing mode:
/// 000 000 0000000 000
///  ^   ^     ^     ^
///  |   |     |     |
///  |  /     /     /
///  | /     /     /
///  | |    /     /
///  | |   /     /
///  | |  /     /
///  | | |     /
///  | | |    /
///  | | |   /
///  | | |  |
///  | | |  > 3 LSB are the diff status
///  | | > 7 middle bits are unused :(
///  | > these 3 bits are the dest file mode
///  > 3 MSB are the source file mode
#[derive(Debug, Copy, Clone)]
pub struct DiffStatusAndFileMode {
    pub data: u16,
}

pub fn diff_status_to_u16(status: DiffStatus) -> u16 {
    match status {
        DiffStatus::Added => 0,
        DiffStatus::Copied => 1,
        DiffStatus::Deleted => 2,
        DiffStatus::Modified => 3,
        DiffStatus::Renamed => 4,
        DiffStatus::TypeChanged => 5,
        DiffStatus::Unmerged => 6,
        DiffStatus::Unknown => 7,
    }
}

pub fn filemode_to_u16(mode: FileMode) -> u16 {
    match mode {
        FileMode::Empty => 0,
        FileMode::Directory => 1,
        FileMode::RegularNonEx => 2,
        FileMode::RegularNonExGroupWrite => 3,
        FileMode::RegularEx => 4,
        FileMode::SymbolicLink => 5,
        FileMode::GitLink => 6,
        FileMode::Unknown => 7,
    }
}

pub fn u16_to_filemode(u: u16) -> FileMode {
    match u {
        0 => FileMode::Empty,
        1 => FileMode::Directory,
        2 => FileMode::RegularNonEx,
        3 => FileMode::RegularNonExGroupWrite,
        4 => FileMode::RegularEx,
        5 => FileMode::SymbolicLink,
        6 => FileMode::GitLink,
        _ => FileMode::Unknown,
    }
}

pub fn u16_to_diff_status(u: u16) -> DiffStatus {
    match u {
        0 => DiffStatus::Added,
        1 => DiffStatus::Copied,
        2 => DiffStatus::Deleted,
        3 => DiffStatus::Modified,
        4 => DiffStatus::Renamed,
        5 => DiffStatus::TypeChanged,
        6 => DiffStatus::Unmerged,
        _ => DiffStatus::Unknown,
    }
}

impl From<(DiffStatus, FileMode, FileMode)> for DiffStatusAndFileMode {
    fn from(orig: (DiffStatus, FileMode, FileMode)) -> Self {
        let (status, mode_src, mode_dest) = orig;
        let left_bits1: u16 = filemode_to_u16(mode_src);
        let left_bits2: u16 = filemode_to_u16(mode_dest);
        let right_bits: u16 = diff_status_to_u16(status);

        let out = left_bits1 << 13;
        let out = out | (left_bits2 << 10);
        let out = out | right_bits;
        DiffStatusAndFileMode {
            data: out,
        }
    }
}

impl From<DiffStatusAndFileMode> for (DiffStatus, FileMode, FileMode) {
    fn from(orig: DiffStatusAndFileMode) -> Self {
        let right_bits =  orig.data & 0b00000000_00000111;
        let left_bits1 = (orig.data & 0b11100000_00000000) >> 13;
        let left_bits2 = (orig.data & 0b00011100_00000000) >> 10;
        let status = u16_to_diff_status(right_bits);
        let mode_src = u16_to_filemode(left_bits1);
        let mode_dest = u16_to_filemode(left_bits2);
        
        (status, mode_src, mode_dest)
    }
}

#[derive(Debug, Clone)]
pub struct RawBlobSummary {
    pub src_dest_mode_and_status: DiffStatusAndFileMode,
    pub src_sha: u64,
    pub dest_sha: u64,
    pub path_str: String,
}

impl RawBlobSummary {
    pub fn status(&self) -> DiffStatus {
        let (status, _, _) = self.src_dest_mode_and_status.into();
        status
    }
}

pub fn hex_char_to_u64(c: char) -> u64 {
    match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'a' => 10,
        'b' => 11,
        'c' => 12,
        'd' => 13,
        'e' => 14,
        'f' => 15,
        _ => 0,
    }
}

pub fn u64_to_hex_char(u: u64) -> char {
    match u {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'a',
        11 => 'b',
        12 => 'c',
        13 => 'd',
        14 => 'e',
        15 => 'f',
        _ => '0',
    }
}

/// every hex character is 4 bits,
/// so max we can handle without overflowing is 16 hex chars
pub fn hex_to_u64(hex: &str) -> u64 {
    let mut out = 0;
    let mut shift = 0;
    for c in hex.chars().rev() {
        let hex_val = hex_char_to_u64(c);
        let shifted_value = hex_val << shift;
        out += shifted_value;
        shift += 4;
    }
    out
}

pub fn u64_to_hex(u: u64) -> [char; 16] {
    [
        u64_to_hex_char((u & 0xf0_00_00_00_00_00_00_00) >> 60),
        u64_to_hex_char((u & 0x0f_00_00_00_00_00_00_00) >> 56),
        u64_to_hex_char((u & 0x00_f0_00_00_00_00_00_00) >> 52),
        u64_to_hex_char((u & 0x00_0f_00_00_00_00_00_00) >> 48),
        u64_to_hex_char((u & 0x00_00_f0_00_00_00_00_00) >> 44),
        u64_to_hex_char((u & 0x00_00_0f_00_00_00_00_00) >> 40),
        u64_to_hex_char((u & 0x00_00_00_f0_00_00_00_00) >> 36),
        u64_to_hex_char((u & 0x00_00_00_0f_00_00_00_00) >> 32),
        u64_to_hex_char((u & 0x00_00_00_00_f0_00_00_00) >> 28),
        u64_to_hex_char((u & 0x00_00_00_00_0f_00_00_00) >> 24),
        u64_to_hex_char((u & 0x00_00_00_00_00_f0_00_00) >> 20),
        u64_to_hex_char((u & 0x00_00_00_00_00_0f_00_00) >> 16),
        u64_to_hex_char((u & 0x00_00_00_00_00_00_f0_00) >> 12),
        u64_to_hex_char((u & 0x00_00_00_00_00_00_0f_00) >> 8),
        u64_to_hex_char((u & 0x00_00_00_00_00_00_00_f0) >> 4),
        u64_to_hex_char((u & 0x00_00_00_00_00_00_00_0f) >> 0),
    ]
}

pub fn parse_blob_line1(line: &str) -> Option<(&str, &str, &str, &str, &str, String)> {
    let mut items = line.split_ascii_whitespace();
    let src_mode = items.next()?;
    // src_mode should be 7 chars because the first is the colon:
    let src_mode = src_mode.get(1..7)?;
    let dest_mode = items.next()?;
    let src_sha = items.next()?;
    let dest_sha = items.next()?;
    let status = items.next()?;
    let path_str: String = items.collect::<Vec<&str>>().join(" ");

    Some((src_mode, dest_mode, src_sha, dest_sha, status, path_str))
}

pub fn parse_blob_line(line: &str) -> io::Result<RawBlobSummary> {
    let (
        src_mode, dest_mode,
        src_sha, dest_sha,
        status, path_str,
    ) = parse_blob_line1(line).ok_or(ioerr!("Failed to parse blob line: {}", line))?;

    let status = DiffStatus::from_str(status).map_err(|e| ioerr!("{}", e))?;
    let src_mode = FileMode::from_str(src_mode).map_err(|e| ioerr!("{}", e))?;
    let dest_mode = FileMode::from_str(dest_mode).map_err(|e| ioerr!("{}", e))?;
    let src_sha = hex_to_u64(src_sha);
    let dest_sha = hex_to_u64(dest_sha);

    let src_dest_mode_and_status = DiffStatusAndFileMode::from((status, src_mode, dest_mode));

    let out = RawBlobSummary {
        src_dest_mode_and_status,
        src_sha,
        dest_sha,
        path_str,
    };
    
    Ok(out)
}

/// this is called by `iterate_blob_log`. This is what actually
/// parses the git log stream into a commit with blob object.
/// This is useful so that instead of passing a committish
/// and invoking the git log command, you can pass in a stream of git log output,
/// or an in-memory buf reader (which is useful for testing).
/// returns true if the outer function should kill the git log stream
/// which is only relevant for `iterate_blob_log`. If you are passing your own
/// in-memory reader, you can ignore the output.
pub fn iterate_blob_log_from_lines<T, L: BufRead>(
    line_reader: &mut L,
    callback: T,
) -> io::Result<bool>
    where T: FnMut(CommitWithBlobs) -> bool,
{
    let mut cb = callback;
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
                // pass the last fully-parsed commit that we parsed
                // on the previous iteration to the callback and
                // then reset our commit/blobs for the next iteration
                // to fill in
                let commit_with_blobs = CommitWithBlobs {
                    commit: last_commit,
                    blobs: last_blobs,
                };
                let should_exit = cb(commit_with_blobs);
                if should_exit {
                    return Ok(true);
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
            let blob = parse_blob_line(&line)?;
            last_blobs.push(blob);
        }
        buf.clear();
    }

    // we have to call this one more time at the end to make sure
    // we get the last commit
    let commit_with_blobs = CommitWithBlobs {
        commit: last_commit,
        blobs: last_blobs,
    };
    let _ = cb(commit_with_blobs);
    // no point in checking if should_exit because we are exiting here anyway

    Ok(false)
}

/// iterates a list of commits and parses
/// the blob summary of each commit and then passes the commit
/// and blobs to a callback. The callback function returns true if
/// it wants to be done reading from the stream, in which case
/// this function will stop reading from the stream and kill the process.
/// Optionally pass in a number of commits to read including the first
/// one indicated by committish. (this corresponds to git log [...] -n <number-of-commits>)
pub fn iterate_blob_log<T>(
    committish: &str,
    num_commits: Option<usize>,
    callback: T,
) -> io::Result<()>
    where T: FnMut(CommitWithBlobs) -> bool,
{
    // TODO: add the '-m' flag if we want to see merge commits with a full blob diff
    // by default, merge commits do not have a blob summary, which
    // makes it easy to tell which commits are merges or not. this default
    // is desirable 9 times out of 10. not sure when -m would be desired though.
    let mut exec_args = vec![
        "git", "--no-pager", "log", "--no-color", "--raw",
        "--pretty=oneline", committish,
    ];
    let n_str = match num_commits {
        Some(n) => n.to_string(),
        None => "".to_string()
    };
    if ! n_str.is_empty() {
        exec_args.push("-n");
        exec_args.push(&n_str);
    }

    let mut child = exec_helpers::spawn_with_env_ex(
        &exec_args,
        &[], &[],
        Some(Stdio::null()), Some(Stdio::null()), Some(Stdio::piped()),
    )?;

    let stdout = child.stdout.as_mut()
        .ok_or(ioerr!("Failed to get child stdout for reading git log of {}", committish))?;
    let mut stdout_read = BufReader::new(stdout);

    let output = iterate_blob_log_from_lines(&mut stdout_read, callback);
    let (should_kill_child, output) = match output {
        Ok(o) => (o, Ok(o)),
        // if there was an error parsing the blob log lines,
        // we should kill the child just in case to prevent
        // running forever on child.wait()
        Err(e) => (true, Err(e))
    };

    if should_kill_child {
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

    if let Err(e) = output {
        return Err(e);
    }

    Ok(())
}

pub fn pull(
    remote_name: &str,
    remote_branch_name: Option<&str>,
    num_commits: Option<u32>,
) -> Result<(), String> {
    let mut exec_args = vec![
        "git", "pull",
        remote_name,
        remote_branch_name.unwrap_or("HEAD"),
    ];

    let mut _depth_string = String::from("");
    if let Some(n) = num_commits {
        _depth_string = format!("--depth={}", n);
        exec_args.push(_depth_string.as_str());
    }

    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

/// target is the current branch
pub fn merge_branch(
    source_branch: &str,
) -> Result<(), String> {
    let exec_args = vec![
        "git", "merge",
        source_branch
    ];
    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

pub fn make_orphan_branch_and_checkout(
    orphan_branch_name: &str
) -> Result<(), String> {
    let exec_args = vec![
        "git", "checkout",
        "--orphan", orphan_branch_name,
    ];
    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

/// after checking out an orphan branch, gits index
/// will be full of files that exist on the filesystem,
/// and git says they are ready to be added. We want
/// to tell git to delete these files (which is safe to do because
/// they exist in another branch)
pub fn remove_index_and_files() -> Result<(), String> {
    let exec_args = ["git", "rm", "-rf", "."];
    let success = exec_helpers::executed_successfully(&exec_args);
    match success {
        true => Ok(()),
        false => Err("Failed to git rm -rf .".into()),
    }
}

pub fn branch_exists(branch_name: &str) -> bool {
    let branch_ref = format!("refs/heads/{}", branch_name);
    let exec_args = [
        "git", "show-ref", "--verify", "--quiet", branch_ref.as_str()
    ];
    // will return 0 (true) if branch exists , 1 (false) otherwise
    exec_helpers::executed_successfully(&exec_args)
}

pub fn delete_branch(
    branch_name: &str
) -> Result<(), String> {
    let exec_args = [
        "git", "branch", "-D", branch_name,
    ];
    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

pub fn checkout_branch(
    branch_name: &str,
    make_new: bool,
) -> Result<(), String> {
    let mut exec_args = vec![
        "git", "checkout"
    ];
    if make_new {
        exec_args.push("-b");
        exec_args.push(branch_name);
    } else {
        exec_args.push(branch_name);
    }

    match exec_helpers::executed_with_error(&exec_args) {
        None => Ok(()),
        Some(e) => Err(e),
    }
}

pub fn get_current_ref() -> Result<String, String> {
    let exec_args = [
        "git", "rev-parse", "--abbrev-ref", "HEAD"
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                // dont want trailing new line
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_all_commits_from_ref(
    refname: &str
) -> Result<Vec<Commit>, String> {
    // TODO: in the future might want more info than
    // just the hash and summary
    let exec_args = [
        "git", "log", refname, "--format=%H [%p] %s",
    ];
    let mut commits = vec![];
    let out_str = match exec_helpers::execute(&exec_args) {
        Err(e) => return Err(e.to_string()),
        Ok(out) => match out.status {
            0 => out.stdout,
            _ => return Err(out.stderr),
        }
    };

    for line in out_str.lines() {
        // everything before first space is
        // the commit hash. everything after is the summary
        let mut line_split = line.split(" ");
        let hash = line_split.nth(0);
        let hash = if let Some(h) = hash {
            h.to_string()
        } else {
            return Err("Failed to parse hash".into());
        };
        // after we took the hash, we now have
        // something like [parent, parent, ...]
        // if there is only one parent, it will be of form
        // [parent], so we check if this commit is a merge
        // or not
        let is_merge = match line_split.next() {
            None => false,
            Some(s) => !s.contains(']')
        };
        // if we did find a merge, that means we have to
        // advance our line split until we have reached
        // the end of the [parent, parent, ...] list
        if is_merge { loop {
            match line_split.next() {
                None => (),
                Some(s) => if s.contains(']') {
                    break;
                }
            }
        }}

        let summary = line_split.collect::<Vec<&str>>().join(" ");
        commits.push(Commit {
            summary: summary,
            id: Oid { hash },
            is_merge,
        });
    }

    Ok(commits)
}

pub fn get_number_of_commits_in_ref(refname: &str) -> Result<usize, String> {
    let exec_args = [
        "git", "log", refname, "--format=%H",
    ];
    let mut child = exec_helpers::spawn_with_env_ex(
        &exec_args,
        &[], &[],
        None, None, Some(Stdio::piped()),
    ).map_err(|e| e.to_string())?;

    let stdout = child.stdout.as_mut()
        .ok_or(format!("Failed to get child stdout for reading number of commits of {}", refname))?;
    let stdout_read = BufReader::new(stdout);

    let mut num_lines = 0;
    for line in stdout_read.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if ! line.is_empty() {
            num_lines += 1;
        }
    }
    child.wait().map_err(|e| e.to_string())?;

    Ok(num_lines)
}

pub fn get_repo_root() -> Result<String, String> {
    let exec_args = [
        "git", "rev-parse", "--show-toplevel",
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                // dont want trailing new line
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_all_files_in_repo() -> Result<String, String> {
    let exec_args = [
        "git", "ls-tree", "-r", "HEAD", "--name-only", "--full-tree"
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string())
    }
}

pub fn reset_stage() -> Result<String, String> {
    // git reset --hard
    let exec_args = [
        "git", "reset", "--hard"
    ];
    match exec_helpers::execute(&exec_args) {
        Ok(out) => {
            if out.status == 0 {
                Ok(out.stdout.trim_end().into())
            } else {
                Err(out.stderr)
            }
        }
        Err(e) => Err(e.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use io::Cursor;

    #[test]
    fn oid_short_and_long_works() {
        let oid_str = "692ec5536e98fecfb3dfbce61a5d89af5f2eee34";
        let oid = Oid {
            hash: oid_str.into(),
        };
        let oid_short = oid.short();
        assert_eq!(oid_short, "692ec55");
        let oid_long = oid.long();
        assert_eq!(oid_long, oid_str);
    }

    // just see if it panics or not :shrug:
    #[test]
    #[cfg_attr(not(feature = "gittests"), ignore)]
    fn get_all_commits_from_ref_works() {
        let data = get_all_commits_from_ref("HEAD");
        assert!(data.is_ok());
        let data = data.unwrap();
        // this only passes if the test is running from
        // a git repository with more than 1 commit...
        assert!(data.len() > 1);
    }

    #[test]
    fn sha_parsing_works() {
        let sha1 = "de1acaefe87e";
        let shau64 = hex_to_u64(sha1);
        println!("{}", shau64);
        let sha_back = u64_to_hex(shau64);
        let sha_back_str: String = sha_back.iter().collect();
        // the parsed back to string one contains extra leading zeros
        // so we test if its somewhere inside:
        assert!(sha_back_str.contains(sha1));

        let sha2 = "000ff";
        let shau64 = hex_to_u64(sha2);
        assert_eq!(shau64, 255);
    }

    #[test]
    fn parse_blob_line_works() {
        let line = ":100644 000000 1234567 0000000 D file5";
        let parsed = parse_blob_line(line).unwrap();
        println!("{:#?}", parsed);
        assert_eq!(parsed.path_str, "file5");
        assert_eq!(parsed.dest_sha, 0);
        assert!(parsed.src_sha != 0);
        let (status, src_mode, dest_mode): (DiffStatus, FileMode, FileMode) = parsed.src_dest_mode_and_status.into();
        assert!(status == DiffStatus::Deleted);
        assert!(src_mode == FileMode::RegularNonEx);
        assert!(dest_mode == FileMode::Empty);
    }

    #[test]
    fn blob_log_properly_detects_merge_commits() {
        let log_output = "somehash commit message here\n01010101010110 another commit message here";
        let mut cursor = Cursor::new(log_output.as_bytes());
        let mut num_commits_visited = 0;
        let _ = iterate_blob_log_from_lines(&mut cursor, |c| {
            num_commits_visited += 1;
            assert!(c.commit.is_merge);
            false
        }).unwrap();
        assert_eq!(num_commits_visited, 2);
    }

    #[test]
    fn blob_log_properly_parses_blobs() {
        let log_output = "hash1 msg1\n:100644 100644 xyz abc M file1.txt\n:100644 00000 123 000 D file2.txt";
        let mut cursor = Cursor::new(log_output.as_bytes());
        let mut num_commits_visited = 0;
        let _ = iterate_blob_log_from_lines(&mut cursor, |c| {
            num_commits_visited += 1;
            assert!(!c.commit.is_merge);
            assert_eq!(c.blobs.len(), 2);
            assert_eq!(c.blobs[0].status(), DiffStatus::Modified);
            assert_eq!(c.blobs[1].status(), DiffStatus::Deleted);
            false
        }).unwrap();
        assert_eq!(num_commits_visited, 1);
    }
}
