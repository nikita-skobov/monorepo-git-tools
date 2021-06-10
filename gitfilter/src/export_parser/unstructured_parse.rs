use std::io::{BufReader, Error, ErrorKind, BufRead, Read, self};
use std::{path::Path, process::Stdio, fmt::Display};
use crate::{ioerr, ioerre};

pub enum ParseState {
    BeforeData,
    Data(usize),
    AfterData,
}

pub struct UnparsedFastExportObject {
    pub before_data_str: String,
    pub data: Vec<u8>,
    pub after_data_str: String,
}

pub type StrOption<'a> = Option<&'a str>;

pub fn make_expected_progress_string(progress_num: u32) -> String {
    let mut s = String::with_capacity(32);
    s.push_str("progress ");
    s.push_str(&progress_num.to_string());
    s.push_str(" objects");
    s
}

// TODO:
// add a way to allow parsing from a read stream, and not necessarily
// opening a new git fast-export command.
// this would be useful to read from a file that contains
// the fast-export text without running the command
// repeatedly for every time you want to split.

/// This 'parser' will only parse the data section
/// and put the rest of the info into a 'metadata' string
/// for future parsing. the rationale is that we need to parse the data section
/// seperately anyway since we need to know when to resume parsing the other
/// sections.
/// optionally specify a path to the
/// git repo if you are not currently in it.
pub fn parse_git_filter_export_with_callback<O, E: Display, P: AsRef<Path>>(
    export_branch: Option<String>,
    with_blobs: bool,
    repo_location: Option<P>,
    cb: impl FnMut(UnparsedFastExportObject) -> Result<O, E>,
) -> io::Result<()> {
    // let now = Instant::now();
    let export_branch = export_branch.unwrap_or("master".into());
    let mut fast_export_command = vec!["git", "fast-export", "--show-original-ids",
        "--signed-tags=strip", "--tag-of-filtered-object=drop",
        "--fake-missing-tagger","--reference-excluded-parents",
        "--reencode=yes", "--use-done-feature", &export_branch,
        "--progress", "1"
    ];
    if !with_blobs {
        fast_export_command.push("--no-data");
    }

    let mut child = exechelper::spawn_with_env_ex2(
        &fast_export_command, &[], &[], repo_location,
        Some(Stdio::null()), Some(Stdio::null()), Some(Stdio::piped()),
    )?;

    let child_stdout = match child.stdout.take() {
        Some(s) => s,
        None => return ioerre!("failed to take child.stdout"),
    };

    let mut cb = cb;
    let mut parse_state = ParseState::BeforeData;
    let mut expected_object = 1;
    let mut expected_progress_string = make_expected_progress_string(expected_object);
    let mut bufreader = BufReader::new(child_stdout);
    // let mut bufreader = BufReader::new(child_stdout).lines();
    
    let mut before_data_str = String::new();
    let mut data_vec: Vec<u8> = vec![];
    let mut after_data_str = String::new();

    loop {
        match parse_state {
            ParseState::BeforeData => {
                let mut line_vec = vec![];
                let num_read = bufreader.read_until('\n' as u8, &mut line_vec)?;
                if num_read == 0 { break; }
                line_vec.pop(); // remove trailing slash
                let line = String::from_utf8_lossy(&line_vec[..]);
                if line.starts_with("data ") {
                    let data_size_index = 5; // data + space is 5 chars
                    let data_size = line.get(data_size_index..).ok_or(ioerr!("Failed to parse data line"))?;
                    let data_size: usize = data_size.parse().map_err(|e| ioerr!("Failed to parse data line:\n{}", e))?;
                    parse_state = ParseState::Data(data_size);
                }
                before_data_str.push_str(&line);
                before_data_str.push('\n');
            }
            ParseState::Data(data_size) => {
                // here we just read the exact number of bytes into a byte vec.
                // this data can potentially be binary data, so we dont convert it to
                // a string. instead, the actual object parser will decide what to do here.
                let mut temp_vec = vec![0; data_size];
                bufreader.read_exact(&mut temp_vec)?;
                parse_state = ParseState::AfterData;
                data_vec = temp_vec;
            }
            ParseState::AfterData => {
                let mut line_vec = vec![];
                let num_read = bufreader.read_until('\n' as u8, &mut line_vec)?;
                if num_read == 0 { break; }
                line_vec.pop(); // remove trailing slash
                let line = unsafe { String::from_utf8_unchecked(line_vec) };
                if line.starts_with(&expected_progress_string) {
                    expected_object += 1;
                    expected_progress_string = make_expected_progress_string(expected_object);

                    let unparsed_obj = UnparsedFastExportObject {
                        before_data_str, data: data_vec, after_data_str
                    };
                    match cb(unparsed_obj) {
                        Ok(_) => {},
                        Err(e) => { // TODO: add bound on E that it should be debug?
                            let _ = child.kill();
                            return ioerre!("Error from callback:\n{}\nclosing fast-export stream", e);
                        }
                    }

                    // TODO: handle error from callback
                    // and close stream then return io error ourselves

                    before_data_str = String::new();
                    data_vec = vec![];
                    after_data_str = String::new();
                    parse_state = ParseState::BeforeData;
                } else {
                    after_data_str.push_str(&line);
                    after_data_str.push('\n');
                }
            }
        }
    }

    // eprintln!("Spent {:?} on reading the git stream", now.elapsed());
    Ok(())
}
