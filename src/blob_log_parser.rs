use std::{str::FromStr, io::{self, BufRead}};
use crate::{ioerr, git_helpers3::Commit};

/// see: https://www.git-scm.com/docs/git-diff#_diff_format_for_merges
/// and: https://www.git-scm.com/docs/git-diff#_raw_output_format


/// see: https://www.git-scm.com/docs/git-diff
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
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

impl Default for DiffStatus {
    fn default() -> Self {
        DiffStatus::Unknown
    }
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
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Hash)]
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

impl Default for FileMode {
    fn default() -> Self {
        FileMode::Unknown
    }
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RawBlobSummary {
    pub src_dest_mode_and_status: DiffStatusAndFileMode,
    pub src_sha: u64,
    pub dest_sha: u64,
    pub path_src: String,
    pub path_dest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RawBlobSummaryWithoutPath {
    pub src_dest_mode_and_status: DiffStatusAndFileMode,
    pub src_sha: u64,
    pub dest_sha: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RawBlobSummaryEndState {
    pub sha: u64,
    pub file_mode: FileMode,
    pub status: DiffStatus,
    pub path_str: String, // dest path
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RawBlobSummaryEndStateWithoutPath {
    pub sha: u64,
    pub file_mode: FileMode,
    pub status: DiffStatus,
}

impl From<RawBlobSummary> for RawBlobSummaryWithoutPath {
    fn from(orig: RawBlobSummary) -> Self {
        RawBlobSummaryWithoutPath {
            src_dest_mode_and_status: orig.src_dest_mode_and_status,
            src_sha: orig.src_sha,
            dest_sha: orig.dest_sha,
        }
    }
}

impl From<RawBlobSummary> for RawBlobSummaryEndState {
    fn from(orig: RawBlobSummary) -> Self {
        let (status, src_mode, dest_mode) = orig.src_dest_mode_and_status.into();
        let (use_mode, use_sha) = match status {
            DiffStatus::Deleted => (src_mode, orig.src_sha),
            _ => (dest_mode, orig.dest_sha),
        };
        RawBlobSummaryEndState {
            status,
            sha: use_sha,
            file_mode: use_mode,
            path_str: orig.path_dest,
        }
    }
}

impl From<RawBlobSummary> for RawBlobSummaryEndStateWithoutPath {
    fn from(orig: RawBlobSummary) -> Self {
        let orig: RawBlobSummaryEndState = orig.into();
        RawBlobSummaryEndStateWithoutPath {
            sha: orig.sha,
            file_mode: orig.file_mode,
            status: orig.status,
        }
    }
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



#[derive(Debug, Clone)]
pub struct CommitWithBlobs {
    pub commit: Commit,
    pub blobs: Vec<RawBlobSummary>,
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum BlobLogLine<'a> {
    /// sha, message
    StartOfMergeCommit(&'a str, &'a str),
    /// sha, message
    StartOfCommit(&'a str, &'a str),
    /// src file mode, dest file mode, src sha, dest sha, mode, path string
    /// NOTE: the path string can contain a tab character which indicates
    /// that there is a src path -> dest path in the case of a rename.
    Blob(&'a str, &'a str, &'a str, &'a str, &'a str, &'a str),
}

pub fn parse_blob_log_commit_line(line: &str) -> Option<BlobLogLine> {
    let mut split = line.split(' ');
    let hash = split.next()?;
    let hash_len = hash.len();

    // check if the next word is "(from"
    // which would indicate this might be a merge commit
    let next_word = split.next()?;
    if next_word != "(from" {
        // this is definitely not a merge commit, so we
        // can return now:
        // (add 1 for the space)
        let message = &line[hash_len + 1..];
        return Some(BlobLogLine::StartOfCommit(hash, message));
    }

    // if that word WAS "(from", then check the next word
    // to see if it has the same length as the hash, and ends with
    // a ")". If so, then this is very likely, but not guaranteed,
    // to be a merge commit.. unfortunately I don't think its possible
    // to 100% guarantee this from the way git log --raw -m outputs these lines...
    let next_word = if let Some(w) = split.next() {
        w
    } else {
        // if we failed to find a word after
        // "(from", then its totally possible the user's
        // commit message was literally just "(from"....
        // so we treat this as a regular commit:
        let message = &line[hash_len + 1..];
        return Some(BlobLogLine::StartOfCommit(hash, message));
    };

    // if this is a merge commit, then the line should start with:
    // 80abb12 (from 221ef3c)
    // where those two commit hashes SHOULD be the same length.
    // so we check here if our next word is the length of the last hash + 1
    // (plus 1 for the end parentheses), and we also check
    // if it ends with a parentheses. If it matches both conditions then
    // this is very likely a merge commit:
    let this_word_len = next_word.len();
    if this_word_len == hash_len + 1 && next_word.ends_with(')') {
        // very likely a merge commit, find the index
        // of where this (from xyz...) ends, and get the
        // message after that:
        let first_from_len = 5; // (from
        let spaces_skipped = 3;
        let message_starts_at = hash_len + spaces_skipped + first_from_len + this_word_len;
        let message = line.get(message_starts_at..)?;
        Some(BlobLogLine::StartOfMergeCommit(hash, message))
    } else {
        // it failed the conditions, so this is
        // not a merge commit
        let message = &line[hash_len + 1..];
        Some(BlobLogLine::StartOfCommit(hash, message))
    }
}

pub fn parse_blob_log_line(line: &str) -> Option<BlobLogLine> {
    let starts_with_colon = line.starts_with(':');

    // start of a commit, split it by hash and message:
    if ! starts_with_colon {
        return parse_blob_log_commit_line(line);
    }
    
    // then this is a blob:
    let line_without_colons = &line[1..];
    // the first 5 strings we want are seperated by spaces
    let mut split = line_without_colons.split_ascii_whitespace();
    let src_mode = split.next()?;
    let dest_mode = split.next()?;
    let src_sha = split.next()?;
    let dest_sha= split.next()?;
    let status_str = split.next()?;
    // plus 6 because we skipped 4 spaces when calling .next() 4 times
    // and then one tab was skipped when getting the status_str
    // and also 1 colon in the beginning
    let num_chars_skipped = 4 + 1 + 1;
    let next_start_index = num_chars_skipped
        + src_mode.len() + dest_mode.len() + src_sha.len()
        + dest_sha.len() + status_str.len();

    // now we know where the rest of the string is:
    // note that the path string could contain a tab character
    // which would indicate its a [src] -> [dest] path string,
    // but we wont parse that here.
    let path_str = &line[next_start_index..];
    return Some(BlobLogLine::Blob(src_mode, dest_mode,
        src_sha, dest_sha, status_str, path_str));
}

pub fn parse_blob_log_line_or_error(line: &str) -> io::Result<BlobLogLine> {
    parse_blob_log_line(line)
        .ok_or(ioerr!("Failed to parse blob log line: {}", line))
}

pub fn create_blob_and_insert(
    blob_list: &mut Vec<RawBlobSummary>,
    src_mode: &str,
    dest_mode: &str,
    src_sha: &str,
    dest_sha: &str,
    status: &str,
    path_str: &str,
) -> io::Result<()> {
    let status = DiffStatus::from_str(status).map_err(|e| ioerr!("{}", e))?;
    let src_mode = FileMode::from_str(src_mode).map_err(|e| ioerr!("{}", e))?;
    let dest_mode = FileMode::from_str(dest_mode).map_err(|e| ioerr!("{}", e))?;
    let src_sha = hex_to_u64(src_sha);
    let dest_sha = hex_to_u64(dest_sha);

    let src_dest_mode_and_status = DiffStatusAndFileMode::from((status, src_mode, dest_mode));

    // if theres a tab in the path_str, then that means
    // this blob has a src_path -> dest_path
    let (path_src, path_dest) = if let Some(tab_index) = path_str.find("\t") {
        let src = &path_str[0..tab_index];
        let dest = &path_str[tab_index + 1..];
        (src.to_string(), dest.to_string())
    } else {
        // no tab, therefore its just one path src -> src
        // but we can treat it as src -> dest:
        (path_str.to_string(), path_str.to_string())
    };

    let out = RawBlobSummary {
        src_dest_mode_and_status,
        src_sha,
        dest_sha,
        path_src,
        path_dest,
    };
    blob_list.push(out);

    Ok(())
}

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
        let line_len = line.len();
        let line = if line.ends_with('\n') {
            &line[0..line_len - 1]
        } else { &line };

        let (hash, message, is_merge) = match parse_blob_log_line_or_error(line)? {
            BlobLogLine::StartOfCommit(hash, message) => (hash, message, false),
            BlobLogLine::StartOfMergeCommit(hash, message) => (hash, message, true),
            BlobLogLine::Blob(sm, dm, ssha, dsha, status, pathstr) => {
                create_blob_and_insert(&mut last_blobs,
                    sm, dm, ssha, dsha, status, pathstr)?;
                buf.clear();
                continue;
            }
        };

        // at this point we know the current line is the start of
        // a commit, or the continuation of a previous merge commit.
        // check if this is the start of a new regular commit:
        if ! is_merge {
            // if we need to add the last parsed commit:
            if add_last_commit {
                let commit_with_blobs = CommitWithBlobs {
                    commit: last_commit,
                    blobs: last_blobs,
                };
                let should_exit = cb(commit_with_blobs);
                if should_exit {
                    return Ok(true);
                }
                last_blobs = vec![];
            }

            // now create the new commit:
            last_commit = Commit::new(hash, message.to_string(), is_merge);
            add_last_commit = true;
        } else {
            // this is a merge commit. check if its NOT a continuation
            // of the previous commit:
            if add_last_commit && last_commit.id.hash != hash {
                let commit_with_blobs = CommitWithBlobs {
                    commit: last_commit,
                    blobs: last_blobs,
                };
                let should_exit = cb(commit_with_blobs);
                if should_exit {
                    return Ok(true);
                }
                last_blobs = vec![];
                last_commit = Commit::new(hash, message.to_string(), is_merge);
            }
            // check if this is the very first commit, and also a merge commit:
            if ! add_last_commit {
                last_commit = Commit::new(hash, message.to_string(), is_merge);
            }
            // otherwise its a continuation of the last merge commit,
            // so no need to do anything
            add_last_commit = true;
        }


        buf.clear();
    }

    if add_last_commit {
        // we have to call this one more time at the end to make sure
        // we get the last commit
        let commit_with_blobs = CommitWithBlobs {
            commit: last_commit,
            blobs: last_blobs,
        };
        let _ = cb(commit_with_blobs);
        // no point in checking if should_exit because we are exiting here anyway
    }

    Ok(false)
}


#[cfg(test)]
mod test {
    use super::*;
    use io::Cursor;


    #[test]
    fn blob_log_line_parse_works() {
        let line = ":000000 100644 0000000 72943a1 A\tmy lib/aaa.txt";
        let parsed = parse_blob_log_line(line).unwrap();
        let expected = BlobLogLine::Blob("000000", "100644", "0000000", "72943a1", "A", "my lib/aaa.txt");
        assert_eq!(parsed, expected);

        // test that rename src->dest still contains both parts:
        let line = ":000000 100644 0000000 72943a1 R100\ta.txt\tb.txt";
        let parsed = parse_blob_log_line(line).unwrap();
        let expected = BlobLogLine::Blob("000000", "100644", "0000000", "72943a1", "R100", "a.txt\tb.txt");
        assert_eq!(parsed, expected);

        // can detect merge commit lines:
        let line = "870fa38 (from 8a6f9ea) Merge branches tmp1 into tmp2";
        let parsed = parse_blob_log_line(line).unwrap();
        let expected = BlobLogLine::StartOfMergeCommit("870fa38", "Merge branches tmp1 into tmp2");
        assert_eq!(parsed, expected);

        // not a merge commit if the second part after '(from' isnt
        // a hash:
        let line = "870fa38 (from something else)";
        let parsed = parse_blob_log_line(line).unwrap();
        let expected = BlobLogLine::StartOfCommit("870fa38", "(from something else)");
        assert_eq!(parsed, expected);
    }

    #[test]
    fn blob_and_commit_parse_works1() {
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

    #[test]
    fn blob_and_commit_parse_works2() {
        let log_output = [
            "870fa38 (from 31be531) Merge branches tmp1 into tmp2",
            ":000000 100644 0000000 3b0e234 A\to2.txt",
            ":000000 100644 0000000 e1ed608 A\to3.txt",
            "870fa38 (from 31be531) Merge branches tmp1 into tmp2",
            ":000000 100644 0000000 3b0e234 A\to2.txt",
        ];
        let log_output = log_output.join("\n");
        let mut cursor = Cursor::new(log_output.as_bytes());
        let mut num_commits_visited = 0;
        let _ = iterate_blob_log_from_lines(&mut cursor, |c| {
            num_commits_visited += 1;
            assert!(c.commit.is_merge);
            assert_eq!(c.blobs.len(), 3);
            assert_eq!(c.blobs[0].status(), DiffStatus::Added);
            false
        }).unwrap();
        assert_eq!(num_commits_visited, 1);
    }
}