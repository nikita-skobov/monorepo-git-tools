use super::UnparsedFastExportObject;
use regex::Regex;
use regex::Captures;
use once_cell::sync::OnceCell;
use std::str::SplitWhitespace;

macro_rules! regex_capture {
    ($text:tt, $reg:tt) => {
        {
            static RE: OnceCell<Regex> = OnceCell::new();
            let re = RE.get_or_init(|| {
                Regex::new($reg).unwrap()
            });
            re.captures($text)
        }
    };
}

pub fn get_regex_authorline(text: &str) -> Option<Captures> {
    regex_capture!(text, r"^(?:author|committer) (.*?) ?<(.*?)> (.*?)$")
}

pub fn get_regex_filemodifyline(text: &str) -> Option<Captures> {
    regex_capture!(text, r"^M ([0-9]*) (.*?) (.*?)$")
}

pub fn get_regex_filedeleteline(text: &str) -> Option<Captures> {
    regex_capture!(text, r"^D (.*?)$")
}

pub fn get_regex_filecopyline(text: &str) -> Option<Captures> {
    regex_capture!(text, r"^C (.*?) (.*?)$")
}

pub fn get_regex_filerenameline(text: &str) -> Option<Captures> {
    regex_capture!(text, r"^R (.*?) (.*?)$")
}

pub fn get_regex_notemodifyline(text: &str) -> Option<Captures> {
    regex_capture!(text, r"^N (.*?) (.*?)$")
}

pub fn owned_string_option(orig: Option<&str>) -> Option<String> {
    match orig {
        Some(s) => Some(s.into()),
        None => None
    }
}

#[derive(Debug, Default)]
pub struct StructuredCommit {
    pub commit_ref: String,
    pub mark: Option<String>,

    // we require it because we pass --show-original-ids to fast-export
    pub original_oid: String,
    pub committer: CommitPersonOwned,
    pub author: AuthorPerson,
    // this is both the header and summary of the commit message
    pub commit_message: String,

    pub from: Option<String>,
    pub merges: Vec<String>,
    pub fileops: Vec<FileOpsOwned>,
}

impl StructuredCommit {
    pub fn get_author(&self) -> Option<&CommitPersonOwned> {
        match self.author {
            AuthorPerson::NoAuthor => None,
            AuthorPerson::SameAsCommitPerson => Some(&self.committer),
            AuthorPerson::Author(ref a) => Some(a),
        }
    }
}

#[derive(Debug, Default)]
pub struct StructuredBlob {
    pub mark: Option<String>,
    pub original_oid: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum StructuredObjectType {
    Blob(StructuredBlob),
    Commit(StructuredCommit),
    NoType,
}

impl Default for StructuredObjectType {
    fn default() -> Self {
        StructuredObjectType::Commit(StructuredCommit::default())
    }
}

#[derive(Default, Debug)]
pub struct StructuredExportObject {
    pub has_reset: Option<String>,
    pub has_reset_from: Option<String>,

    // there are other features but we dont implement them,
    // if we see the keyword 'feature', we assume its "feature done"
    pub has_feature_done: bool,

    pub data_size: String,

    pub object_type: StructuredObjectType,
}

pub enum BeforeDataParserMode {
    Initial,
    Reset,
    Commit,
    Blob,
}
use BeforeDataParserMode::*;

pub enum AfterDataParserMode {
    Initial,
    AfterFrom,
    AfterMerge,
}
use AfterDataParserMode::*;

pub enum NextWordType {
    Oid,
    Mark,
    CommitRef,
    ResetFrom,
    ResetLine,
    Data,
    From,
    Merge,
}
use NextWordType::*;

/// here we diverge from git-fast-import spec a bit.
/// the fast-import spec has several commands, but we only handle
/// two of them: commit and blob.
/// we dont handle tags because we ignore tags. resets are part
/// of the before_data_object so we dont treat it as a seperate object,
/// same goes for feature done. we ignore progress, checkpoint and alias, and the rest
#[derive(Debug)]
pub enum ObjectType<'a> {
    Commit(CommitObject<'a>),
    Blob(BlobObject<'a>),
}

impl<'a> Default for ObjectType<'a> {
    fn default() -> Self {
        ObjectType::Commit(CommitObject::default())
    }
}

#[derive(Default, Debug)]
pub struct CommitPerson<'a> {
    pub name: Option<&'a str>,
    pub email: &'a str,
    pub timestr: &'a str,
}

#[derive(Default, Debug)]
pub struct CommitPersonOwned {
    pub name: Option<String>,
    pub email: String,
    pub timestr: String,
}

impl<'a> Into<CommitPersonOwned> for &CommitPerson<'a> {
    fn into(self) -> CommitPersonOwned {
        CommitPersonOwned {
            name: owned_string_option(self.name),
            email: self.email.into(),
            timestr: self.timestr.into(),
        }
    }
}

impl<'a> PartialEq for CommitPerson<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
        self.email == other.email &&
        self.timestr == other.timestr
    }
}

#[derive(Debug)]
pub enum AuthorPerson {
    NoAuthor,
    SameAsCommitPerson,
    Author(CommitPersonOwned),
}
impl Default for AuthorPerson {
    fn default() -> Self {
        AuthorPerson::NoAuthor
    }
}

#[derive(Debug, Default)]
pub struct BlobObject<'a> {
    mark: Option<&'a str>,
    oid: &'a str,
}

#[derive(Default, Debug)]
pub struct CommitObject<'a> {
    refname: &'a str,
    mark: Option<&'a str>,
    // technically this is optional, but the way we call git-fast-export
    // we should always be given an oid
    oid: &'a str,

    author: Option<CommitPerson<'a>>,
    committer: CommitPerson<'a>,
}

#[derive(Default, Debug)]
pub struct BeforeDataObject<'a> {
    has_reset: Option<&'a str>,
    has_reset_from: Option<&'a str>,

    // there are other features but we dont implement them,
    // if we see the keyword 'feature', we assume its "feature done"
    has_feature_done: bool,

    object: ObjectType<'a>,

    data: &'a str,
}

#[derive(Debug)]
pub enum FileOps<'a> {
    FileModify(&'a str, &'a str, &'a str),
    FileDelete(&'a str),
    FileCopy(&'a str, &'a str),
    FileRename(&'a str, &'a str),
    FileDeleteAll,
    NoteModify(&'a str, &'a str),
}
#[derive(Debug, PartialEq)]
pub enum FileOpsOwned {
    FileModify(String, String, String),
    FileDelete(String),
    FileCopy(String, String),
    FileRename(String, String),
    FileDeleteAll,
    NoteModify(String, String),
}
impl Default for FileOpsOwned {
    fn default() -> Self {
        FileOpsOwned::FileDeleteAll
    }
}

impl<'a> Into<FileOpsOwned> for &FileOps<'a> {
    fn into(self) -> FileOpsOwned {
        match self {
            FileOps::FileModify(a, b, c) => FileOpsOwned::FileModify((*a).into(), (*b).into(), (*c).into()),
            FileOps::FileDelete(a) => FileOpsOwned::FileDelete((*a).into()),
            FileOps::FileCopy(a, b) => FileOpsOwned::FileCopy((*a).into(), (*b).into()),
            FileOps::FileRename(a, b) => FileOpsOwned::FileRename((*a).into(), (*b).into()),
            FileOps::NoteModify(a, b) => FileOpsOwned::NoteModify((*a).into(), (*b).into()),
            FileOps::FileDeleteAll => FileOpsOwned::FileDeleteAll,
        }
    }
}

#[derive(Default, Debug)]
pub struct AfterDataObject<'a> {
    from: Option<&'a str>,

    merges: Vec<&'a str>,

    fileops: Vec<FileOps<'a>>,
}

pub fn set_object_property<'a>(
    value: &'a str,
    object: &mut BeforeDataObject<'a>,
    next_word_type: NextWordType,
) {
    if let ObjectType::Commit(commit_obj) = &mut object.object {
        if let Oid = next_word_type {
            commit_obj.oid = value;
        } else if let Mark = next_word_type {
            commit_obj.mark = Some(value);
        }
    } else if let ObjectType::Blob(blob_obj) = &mut object.object {
        if let Oid = next_word_type {
            blob_obj.oid = value;
        } else if let Mark = next_word_type {
            blob_obj.mark = Some(value);
        }
    }
}

// Most parsing just needs to see the next word
// this method handles all parsing that only needs to take a single
// word and put it into some property. the property thats being updated
// depends on the value of next_word_type
pub fn parse_next_word<'a>(
    word_split: &mut SplitWhitespace<'a>,
    object: &mut BeforeDataObject<'a>,
    next_word_type: NextWordType,
    parse_mode: &mut BeforeDataParserMode,
) -> Option<()> {
    let next_word = word_split.next()?;
    match next_word_type {
        Oid | Mark => set_object_property(next_word, object, next_word_type),
        CommitRef => {
            let mut commit_obj = CommitObject::default();
            commit_obj.refname = next_word;
            object.object = ObjectType::Commit(commit_obj);
            *parse_mode = BeforeDataParserMode::Commit;
        },
        ResetFrom => {
            object.has_reset_from = Some(next_word);
            *parse_mode = BeforeDataParserMode::Initial;
        },
        ResetLine => {
            // might need to get rid of this check?
            // Im not sure if its possible to see multiple reset
            // lines in a row. If it is, then our parser cannot handle that.
            // if we need to handle this, then wed modify the BeforeDataObject
            // to have a Vec<ResetInfo>
            if object.has_reset.is_some() {
                panic!("This object already has a reset?");
            }
            object.has_reset = Some(next_word);
            *parse_mode = BeforeDataParserMode::Reset;
        },
        Data => {
            object.data = next_word;
        },
        // not relevant to the before data object
        _ => {},
    }
    Some(())
}


// this is used for parsing the next word but
// only for the after data object
pub fn parse_next_word2<'a>(
    word_split: &mut SplitWhitespace<'a>,
    object: &mut AfterDataObject<'a>,
    next_word_type: NextWordType,
    parse_mode: &mut AfterDataParserMode,
) -> Option<()> {
    let next_word = word_split.next()?;
    match next_word_type {
        From => {
            object.from = Some(next_word);
            *parse_mode = AfterFrom;
        },
        Merge => {
            object.merges.push(next_word);
            *parse_mode = AfterFrom;
        }
        // the rest of the next word types are not relevant
        // to the after data object
        _ => {},
    }
    Some(())
}

pub fn parse_author_or_committer_line<'a>(
    line: &'a str,
    object: &mut BeforeDataObject<'a>,
    is_author: bool,
) -> Option<()> {
    let captures = get_regex_authorline(line)?;
    let name = captures.get(1)?.as_str();
    let email = captures.get(2)?.as_str();
    let timestr = captures.get(3)?.as_str();

    let person = CommitPerson {
        name: if name.is_empty() { None } else { Some(name) },
        email,
        timestr,
    };
    if let ObjectType::Commit(commit_obj) = &mut object.object {
        if is_author {
            commit_obj.author = Some(person);
        } else {
            commit_obj.committer = person;
        }
    }

    Some(())
}

pub fn parse_filemodify_line<'a>(
    line: &'a str,
    object: &mut AfterDataObject<'a>,
    parse_mode: &mut AfterDataParserMode,
) -> Option<()> {
    let captures = get_regex_filemodifyline(line).unwrap();
    let mode = captures.get(1)?.as_str();
    let dataref = captures.get(2)?.as_str();
    let path = captures.get(3)?.as_str();

    let fileop = FileOps::FileModify(mode, dataref, path);

    object.fileops.push(fileop);
    *parse_mode = AfterMerge;

    Some(())
}

pub fn parse_filedelete_line<'a>(
    line: &'a str,
    object: &mut AfterDataObject<'a>,
    parse_mode: &mut AfterDataParserMode,
) -> Option<()> {
    let captures = get_regex_filedeleteline(line)?;
    let path = captures.get(1)?.as_str();

    let fileop = FileOps::FileDelete(path);

    object.fileops.push(fileop);
    *parse_mode = AfterMerge;

    Some(())
}

pub fn parse_filecopy_line<'a>(
    line: &'a str,
    object: &mut AfterDataObject<'a>,
    parse_mode: &mut AfterDataParserMode,
) -> Option<()> {
    let captures = get_regex_filecopyline(line)?;
    let src_path = captures.get(1)?.as_str();
    let dest_path = captures.get(2)?.as_str();

    let fileop = FileOps::FileCopy(src_path, dest_path);

    object.fileops.push(fileop);
    *parse_mode = AfterMerge;

    Some(())
}

pub fn parse_filerename_line<'a>(
    line: &'a str,
    object: &mut AfterDataObject<'a>,
    parse_mode: &mut AfterDataParserMode,
) -> Option<()> {
    let captures = get_regex_filerenameline(line)?;
    let src_path = captures.get(1)?.as_str();
    let dest_path = captures.get(2)?.as_str();

    let fileop = FileOps::FileRename(src_path, dest_path);

    object.fileops.push(fileop);
    *parse_mode = AfterMerge;

    Some(())
}

pub fn parse_notemodify_line<'a>(
    line: &'a str,
    object: &mut AfterDataObject<'a>,
    parse_mode: &mut AfterDataParserMode,
) -> Option<()> {
    let captures = get_regex_notemodifyline(line)?;
    let dataref = captures.get(1)?.as_str();
    let commitish = captures.get(2)?.as_str();

    let fileop = FileOps::NoteModify(dataref, commitish);

    object.fileops.push(fileop);
    *parse_mode = AfterMerge;

    Some(())
}

pub fn parse_before_data_line<'a>(
    line: &'a str,
    parse_mode: &mut BeforeDataParserMode,
    object: &mut BeforeDataObject<'a>,
) -> Option<()> {
    let mut word_split = line.split_whitespace();
    let first_word = word_split.next()?;

    match parse_mode {
        // in the initial state we are looking for one of several words
        // feature, reset, commit, or blob
        BeforeDataParserMode::Initial => match first_word {
            "feature" => object.has_feature_done = true,
            "reset" => parse_next_word(&mut word_split, object, ResetLine, parse_mode)?,
            "commit" => parse_next_word(&mut word_split, object, CommitRef, parse_mode)?,
            "blob" => {
                object.object = ObjectType::Blob(BlobObject::default());
                *parse_mode = Blob;
            }
            _ => panic!("Unknown initial parsing?\n{}", line),
        },

        // if we are not in initial parsing mode, then we are parsing
        // reset info, commit info, or blob info.

        // reset is a boring parse because 9999% of the time there is no from <commit-ish>
        // so usually this will just rever back to initial parse mode. but if we do have
        // a from, we check for it here.
        Reset => match first_word {
            "from" => parse_next_word(&mut word_split, object, ResetFrom, parse_mode)?,
            "commit" => parse_next_word(&mut word_split, object, CommitRef, parse_mode)?,
            _ => panic!("Unknown reset parsing?\n{}", line),
        },

        // commit has a lot of stuff to parse out
        Commit => match first_word {
            "mark" => parse_next_word(&mut word_split, object, Mark, parse_mode)?,
            "original-oid" => parse_next_word(&mut word_split, object, Oid, parse_mode)?,
            "author" => parse_author_or_committer_line(line, object, true)?,
            "committer" => parse_author_or_committer_line(line, object, false)?,
            // I dont think we need to handle this because we do --reencode=yes
            "encoding" => (),
            "data" => parse_next_word(&mut word_split, object, Data, parse_mode)?,
            _ => panic!("Unknown commit parsing?\n{}", line),
        },

        Blob => match first_word {
            "mark" => parse_next_word(&mut word_split, object, Mark, parse_mode)?,
            "original-oid" => parse_next_word(&mut word_split, object, Oid, parse_mode)?,
            "data" => parse_next_word(&mut word_split, object, Data, parse_mode)?,
            _ => panic!("Unknown blob parsing?\n{}", line),
        }
    }

    Some(())
}

pub fn parse_after_data_line<'a>(
    line: &'a str,
    parse_mode: &mut AfterDataParserMode,
    object: &mut AfterDataObject<'a>,
) -> Option<()> {
    let mut word_split = line.split_whitespace();
    let first_word = word_split.next()?;

    match parse_mode {
        AfterDataParserMode::Initial => match first_word {
            "from" => parse_next_word2(&mut word_split, object, From, parse_mode)?,
            "merge" => parse_next_word2(&mut word_split, object, Merge, parse_mode)?,
            "M" => parse_filemodify_line(line, object, parse_mode)?,
            "D" => parse_filedelete_line(line, object, parse_mode)?,
            "C" => parse_filecopy_line(line, object, parse_mode)?,
            "R" => parse_filerename_line(line, object, parse_mode)?,
            "N" => parse_notemodify_line(line, object, parse_mode)?,
            "deleteall" => {
                object.fileops.push(FileOps::FileDeleteAll);
                *parse_mode = AfterMerge;
            }
            _ => panic!("Unknown after data parsing?\n{}", line),
        },
        // if we have already seen a 'from' keyword
        // then that cannot appear again, so we dont
        // bother checking for it again
        AfterFrom => match first_word {
            "merge" => parse_next_word2(&mut word_split, object, Merge, parse_mode)?,
            "M" => parse_filemodify_line(line, object, parse_mode)?,
            "D" => parse_filedelete_line(line, object, parse_mode)?,
            "C" => parse_filecopy_line(line, object, parse_mode)?,
            "R" => parse_filerename_line(line, object, parse_mode)?,
            "N" => parse_notemodify_line(line, object, parse_mode)?,
            "deleteall" => {
                object.fileops.push(FileOps::FileDeleteAll);
                *parse_mode = AfterMerge;
            }
            _ => panic!("Unknown after data parsing?\n{}", line),
        },

        // if we have gotten past merge, then we only need to look at potential fileops
        AfterMerge => match  first_word {
            "M" => parse_filemodify_line(line, object, parse_mode)?,
            "D" => parse_filedelete_line(line, object, parse_mode)?,
            "C" => parse_filecopy_line(line, object, parse_mode)?,
            "R" => parse_filerename_line(line, object, parse_mode)?,
            "N" => parse_notemodify_line(line, object, parse_mode)?,
            "deleteall" => {
                object.fileops.push(FileOps::FileDeleteAll);
                *parse_mode = AfterMerge;
            }
            _ => panic!("Unknown after data parsing?\n{}", line),
        },
    }

    Some(())
}

pub fn parse_before_data<'a>(before_data_str: &'a String) -> Option<BeforeDataObject<'a>> {
    let mut parser_mode = BeforeDataParserMode::Initial;
    let mut output_obj = BeforeDataObject::default();
    for line in before_data_str.lines() {
        if line.is_empty() { continue; }
        parse_before_data_line(line, &mut parser_mode, &mut output_obj)?;
    }

    Some(output_obj)
}

pub fn parse_after_data<'a>(after_data_str: &'a String) -> Option<AfterDataObject<'a>> {
    let mut parser_mode = AfterDataParserMode::Initial;
    let mut output_obj = AfterDataObject::default();

    for line in after_data_str.lines() {
        if line.is_empty() { continue; }
        parse_after_data_line(line, &mut parser_mode, &mut output_obj)?;
    }

    Some(output_obj)
}

pub fn parse_into_structured_object(unparsed: UnparsedFastExportObject) -> StructuredExportObject {
    // print!("{}", unparsed.before_data_str);
    // print!("{}", unparsed.after_data_str);
    let before_data_obj = parse_before_data(&unparsed.before_data_str).expect("Failed to parse before data section");
    let after_data_obj = parse_after_data(&unparsed.after_data_str).expect("Failed to parse after data section");
    
    // println!("---------------------");
    // println!("{:?}", before_data_obj);
    // println!("{:?}", after_data_obj);
    // println!("============================");

    let mut output_object = StructuredExportObject::default();
    output_object.has_feature_done = before_data_obj.has_feature_done;
    output_object.has_reset = owned_string_option(before_data_obj.has_reset);
    output_object.has_reset_from = owned_string_option(before_data_obj.has_reset_from);
    output_object.data_size = before_data_obj.data.into();

    let object_type = match &before_data_obj.object {
        ObjectType::Commit(commit_obj) => {
            let author_type = match &commit_obj.author {
                None => AuthorPerson::NoAuthor,
                Some(author) => {
                    if commit_obj.committer == *author {
                        AuthorPerson::SameAsCommitPerson
                    } else {
                        AuthorPerson::Author(author.into())
                    }
                }
            };
    
            let structured_commit = StructuredCommit {
                commit_ref: commit_obj.refname.into(),
                mark: owned_string_option(commit_obj.mark),
                original_oid: commit_obj.oid.into(),
                committer: (&commit_obj.committer).into(),
                author: author_type,
                commit_message: String::from_utf8_lossy(&unparsed.data).into(),
                from: owned_string_option(after_data_obj.from),
                merges: after_data_obj.merges.iter().map(|x| String::from(*x)).collect(),
                fileops: after_data_obj.fileops.iter().map(|x| x.into()).collect(),
            };
            StructuredObjectType::Commit(structured_commit)
        }
        ObjectType::Blob(blob_obj) => {
            let structured_blob = StructuredBlob {
                mark: owned_string_option(blob_obj.mark), 
                original_oid: blob_obj.oid.into(),
                data: unparsed.data,
            };
            StructuredObjectType::Blob(structured_blob)
        }
    };

    output_object.object_type = object_type;

    // println!("{:#?}", output_object);

    output_object
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn before_data_object_works1() {
        let test_str = r#"
feature done
reset refs/heads/master
commit refs/heads/master
mark :1
original-oid 0c0dffba54e509a82483be2f78bf09451d03babb
author Bryan Bryan <bb@email.com> 1548162866 -0800
committer Bryan Bryan <bb@email.com> 1548162866 -0800
data 12"#;

        let test_string = String::from(test_str);
        let before_obj = parse_before_data(&test_string).unwrap();
        // println!("{:#?}", before_obj);

        assert_eq!(before_obj.has_feature_done, true);
        assert_eq!(before_obj.has_reset, Some("refs/heads/master"));
        assert_eq!(before_obj.data, "12");
        let obj = if let ObjectType::Commit(c) = before_obj.object {
            c
        } else { panic!("expected commit object") };
        assert_eq!(obj.committer.name, Some("Bryan Bryan"));
        assert_eq!(obj.committer.email, "bb@email.com");
        assert_eq!(obj.author.unwrap().timestr, "1548162866 -0800");
    }

    #[test]
    fn regex_author_capture_works() {
        let sample1 = "author Bryan Bryan <bb@email.com> 1548162866 -0800";
        let captures = get_regex_authorline(sample1).unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "Bryan Bryan");
        assert_eq!(captures.get(2).unwrap().as_str(), "bb@email.com");
        assert_eq!(captures.get(3).unwrap().as_str(), "1548162866 -0800");

        // it also works if the starting word is committer
        // and the name can be optional
        let sample2 = "committer <bb@email.com> 1548162866 -0800";
        let captures = get_regex_authorline(sample2).unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "");
        assert_eq!(captures.get(2).unwrap().as_str(), "bb@email.com");
        assert_eq!(captures.get(3).unwrap().as_str(), "1548162866 -0800");

        // found this in linux git history
        // note the ß character here is encoded differently in this string
        // than it is when we get it from git...
        let sample3 = "author Albrecht Dreß <albrecht.dress@com.rmk.(none)> 1117828346 +0100";
        let captures = get_regex_authorline(sample3).unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "Albrecht Dreß");
        assert_eq!(captures.get(2).unwrap().as_str(), "albrecht.dress@com.rmk.(none)");
        assert_eq!(captures.get(3).unwrap().as_str(), "1117828346 +0100");
    }

    #[test]
    fn regex_ilemodify_works() {
        let sample1 = "M 100644 dd82933dd7b005c2b3137ffd8c28710c2ecc1e2a lib/rust/.gitignore";
        let captures = get_regex_filemodifyline(sample1).unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "100644");
    }
}
