use super::export_parser;
use export_parser::{CommitObject, StructuredExportObject, StructuredCommit};
use export_parser::FileOpsOwned;
use std::io::Write;
use std::io;

pub enum FilterRule {
    FilterRulePathInclude(String),
    FilterRulePathExclude(String),
}
pub use FilterRule::*;

pub type FilterRules = Vec<FilterRule>;

/// Filter options are
/// just the initial options passed to initiate
/// the filtering operation. the actual
/// rules that determine how something is filtered is in
/// `FilterRules`
#[derive(Debug, Default)]
pub struct FilterOptions<T: Write> {
    pub stream: T,
    /// defaults to master
    pub branch: Option<String>,
    pub with_blobs: bool,
    // TODO:
    // pub num_threads: Option<usize>,
}

impl<T: Write> From<T> for FilterOptions<T> {
    fn from(orig: T) -> Self {
        FilterOptions {
            stream: orig,
            branch: None,
            with_blobs: false,
        }
    }
}

pub fn should_use_file_modify(
    mode: &mut String,
    dataref: &mut String,
    path: &mut String,
    filter_rules: &FilterRules,
) -> bool {
    let mut should_keep = false;
    for filter_rule in filter_rules {
        match filter_rule {
            FilterRulePathInclude(include) => {
                if path.starts_with(include) {
                    should_keep = true;
                }
            }
            FilterRulePathExclude(exclude) => {
                if path.starts_with(exclude) {
                    should_keep = false;
                }
            }
        }
    }

    should_keep
}

pub fn should_use_file_delete(
    path: &mut String,
    filter_rules: &FilterRules,
) -> bool {
    let mut should_keep = false;
    for filter_rule in filter_rules {
        match filter_rule {
            FilterRulePathInclude(include) => {
                if path.starts_with(include) {
                    should_keep = true;
                }
            }
            FilterRulePathExclude(exclude) => {
                if path.starts_with(exclude) {
                    should_keep = false;
                }
            }
        }
    }

    should_keep
}

pub fn perform_filter(
    have_used_a_commit: bool,
    commit: &mut StructuredCommit,
    filter_rules: &FilterRules,
) -> bool {
    let mut newfileops = vec![];
    for op in commit.fileops.drain(..) {
        match op {
            // TODO: not sure if need to handle these?
            // by not doing anything here, we are explicitly
            // removing them, but file copy
            // is a hard one, not sure what to do for that one
            // is removing that ok?
            FileOpsOwned::FileCopy(_, _) => {}
            FileOpsOwned::FileDeleteAll => {}
            FileOpsOwned::NoteModify(_, _) => {}

            // renames are tricky too, but I think
            // we want to drop them unless
            // we are renaming something WITHIN the
            // include path. ie: both src and dest
            // must contain the path we want to include.
            // otherwise, if we want to include B, and
            // the rename is from A -> B, then why would we include
            // something about A here since we are filtering it out?
            FileOpsOwned::FileRename(src, dest) => {
                // TODO:
                // if src.starts_with(include_path) && dest.starts_with(include_path) {
                //     newfileops.push(FileOpsOwned::FileRename(src, dest));
                // }
            }

            // easiest cases. if it exists, keep it
            FileOpsOwned::FileModify(mut mode, mut dataref, mut path) => {
                if should_use_file_modify(&mut mode, &mut dataref, &mut path, filter_rules) {
                    newfileops.push(FileOpsOwned::FileModify(mode, dataref, path));
                }
            }
            FileOpsOwned::FileDelete(mut path) => {
                if should_use_file_delete(&mut path, filter_rules) {
                    newfileops.push(FileOpsOwned::FileDelete(path));
                }
            }
        }
    }
    if newfileops.is_empty() {
        return false;
    }
    commit.fileops = newfileops;
    // if we havent used a commit yet, but this is our first,
    // then we want this to not have a from line:
    if !have_used_a_commit {
        commit.from = None;
    }
    true
}

pub fn filter_with_rules<T: Write>(
    filter_options: FilterOptions<T>,
    filter_rules: FilterRules,
) -> io::Result<()> {
    eprintln!("Usingsadsa branch: {:?}", filter_options.branch);
    let mut have_used_a_commit = false;
    let cb = |obj: &mut StructuredExportObject| -> bool {
        // TODO: filter on blobs as well:
        match &mut obj.object_type {
            export_parser::StructuredObjectType::Blob(_) => true,
            export_parser::StructuredObjectType::Commit(ref mut c) => {
                let should_use = perform_filter(have_used_a_commit, c, &filter_rules);
                if !have_used_a_commit && should_use {
                    have_used_a_commit = true;
                }
                should_use
            },
        }
    };
    filter_with_cb(filter_options, cb)
}

// temporary function to test out filtering
pub fn filter_with_cb<T: Write, F: Into<FilterOptions<T>>>(
    options: F,
    cb: impl FnMut(&mut StructuredExportObject) -> bool
) -> io::Result<()> {
    let options: FilterOptions<T> = options.into();
    let mut stream = options.stream;
    let mut cb = cb;
    export_parser::parse_git_filter_export_via_channel(options.branch, options.with_blobs,
        |mut obj| {
            if cb(&mut obj) {
                return export_parser::write_to_stream(&mut stream, obj);
            }
            Ok(())
        }
    )?;

    stream.write_all(b"done\n")?;

    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;
    use std::io::sink;
    use std::io::Cursor;
    use std::io::Read;
    use export_parser::StructuredObjectType;

    #[test]
    fn filter_path_works() {
        let writer = sink();
        filter_with_cb(writer, |obj| {
            match &obj.object_type {
                StructuredObjectType::Blob(_) => true,
                StructuredObjectType::Commit(commit_obj) => {
                    if commit_obj.committer.email.contains("jerry") {
                        false
                    } else {
                        true
                    }
                }
            }
        }).unwrap();
    }

    #[test]
    fn can_modify_filter_objects() {
        let mut writer = Cursor::new(vec![]);
        filter_with_cb(&mut writer, |obj| {
            if let Some(reset) = &mut obj.has_reset {
                *reset = "refs/heads/NEWBRANCHNAMEAAAAAAA".into();
            }
            match &mut obj.object_type {
                StructuredObjectType::Blob(_) => true,
                StructuredObjectType::Commit(commit_obj) => {
                    commit_obj.commit_ref = commit_obj.commit_ref.replace("master", "NEWBRANCHNAMEAAAAAAA");
                    true
                }
            }
        }).unwrap();

        let mut s = String::from("");
        writer.set_position(0);
        writer.read_to_string(&mut s).unwrap();
        assert!(s.contains("refs/heads/NEWBRANCHNAMEAAAAAAA"));
        assert!(!s.contains("refs/heads/master"));
    }
}
