use super::export_parser;
use export_parser::{StructuredExportObject, StructuredCommit};
use export_parser::FileOpsOwned;
use super::filter_state::FilterState;
use super::filter_state::MAPS_TO_EMPTY;
use super::filter_state::UNKNOWN_MAP;
use std::io::Write;
use std::process::Stdio;
use std::{path::{PathBuf, Path}, io};

#[derive(Clone, Debug)]
pub enum FilterRule {
    FilterRulePathInclude(String),
    FilterRulePathExclude(String),
    FilterRulePathRename(String, String),
}
pub use FilterRule::*;

pub type FilterRules = Vec<FilterRule>;
#[derive(Debug)]
pub struct FilterError(String);

impl From<FilterError> for io::Error {
    fn from(orig: FilterError) -> Self {
        io::Error::new(io::ErrorKind::Other, orig.0)
    }
}

/// how to use this filtered commit
#[derive(Debug)]
pub enum FilterResponse {
    /// dont use it, dont output anything, skip it entirely
    DontUse,
    /// either use it as is, or if it was already modified
    /// by the user, then use what the user modified 
    UseAsIs,

    UseAsReset(FilterAsReset),
}

#[derive(Debug)]
pub enum FilterAsReset {
    /// instead of using this as a commit, use it as a reset
    /// and provide the ref to reset
    AsReset(String),

    /// like AsReset, but in addition to the ref to reset, provide
    /// a from mark to reset from
    AsResetFrom(String, String),
}

impl FilterResponse {
    pub fn is_used(&self) -> bool {
        match self {
            FilterResponse::DontUse => false,
            _ => true,
        }
    }

    pub fn is_a_reset(self) -> Option<FilterAsReset> {
        match self {
            FilterResponse::UseAsReset(r) => Some(r),
            _ => None,
        }
    }
}

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
    pub default_include: bool,
    pub with_blobs: bool,
    // TODO:
    // pub num_threads: Option<usize>,
}

impl<T: Write> From<T> for FilterOptions<T> {
    fn from(orig: T) -> Self {
        FilterOptions {
            stream: orig,
            branch: None,
            default_include: false,
            with_blobs: false,
        }
    }
}

// TODO: originally i had seperate functions for
// each type of opeartion git fast-export could give us
// (ie: modify, rename, delete, etc)
// but since we only care about modify/delete right now
// and the functionality for those is the same, im combining
// them into one method. so in the future if we need
// seperate handling depending on the operation, then split
// this method out
pub fn should_use_file(
    path: &mut String,
    filter_rules: &FilterRules,
    default_include: bool,
) -> bool {
    let pathlen = path.len();
    let (check_path, re_add_quotes) = if path.starts_with('"') && path.ends_with('"') {
        (&path[1..(pathlen - 1)], true)
    } else {
        (&path[..], false)
    };
    let mut should_keep = default_include;
    let mut replace = None;
    for filter_rule in filter_rules {
        match filter_rule {
            FilterRulePathInclude(include) => {
                if check_path.starts_with(include) {
                    should_keep = true;
                }
            }
            FilterRulePathExclude(exclude) => {
                if check_path.starts_with(exclude) {
                    if check_path == exclude {
                        // if it matches exactly, we should not iterate anymore
                        // this is a definitive exclude
                        return false;
                    }
                    should_keep = false;
                }
            }
            FilterRulePathRename(src, dest) => {
                if check_path.starts_with(src) {
                    replace = Some(check_path.replacen(src, dest, 1));
                    should_keep = true;
                }
            }
        }
    }

    // we want to handle path replace after everything else.
    // consider the case:
    // FilterExclude (src/a.txt)
    // FilterPathRename (src/ -> lib/)
    // then say, we are deciding if we want to include src/a.txt
    // since the path rename comes 
    if let Some(replace_with) = replace {        
        if should_keep {
            *path = replace_with;
        }
    }
    // if git fast-export sees a path that has a space, it wraps it in quotes
    // but for our pattern matching above, it would be easier if it didnt have spaces
    // so after we filter, if we still want to keep this, and it still has
    // spaces, then we have to readd quotes to the ends of the path
    if should_keep && re_add_quotes && path.contains(' ') {
        *path = format!("\"{}\"", path);
    }

    should_keep
}

pub fn apply_filter_rules_to_fileops(
    default_include: bool,
    _filter_state: &mut FilterState,
    commit: &mut StructuredCommit,
    filter_rules: &FilterRules,
) -> Vec<FileOpsOwned> {
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
            FileOpsOwned::FileRename(_, _) => {
                // TODO:
                // if src.starts_with(include_path) && dest.starts_with(include_path) {
                //     newfileops.push(FileOpsOwned::FileRename(src, dest));
                // }
            }

            // easiest cases. if it exists, keep it
            FileOpsOwned::FileModify(mode, dataref, mut path) => {
                if should_use_file(&mut path, filter_rules, default_include) {
                    newfileops.push(FileOpsOwned::FileModify(mode, dataref, path));
                }
            }
            FileOpsOwned::FileDelete(mut path) => {
                if should_use_file(&mut path, filter_rules, default_include) {
                    newfileops.push(FileOpsOwned::FileDelete(path));
                }
            }
        }
    }
    newfileops
}

pub fn get_mapped_mark(
    filter_state: &mut FilterState,
    mark: usize
) -> Result<usize, FilterError> {
    let mapped_to = match filter_state.get_mapped_mark(mark) {
        Some(m) => if *m == UNKNOWN_MAP { None } else { Some(*m) },
        None => None,
    };
    match mapped_to {
        Some(m) => Ok(m),
        None => {
            let err_str = format!("Failed to find appropriate mapping of mark {}", mark);
            Err(FilterError(err_str))
        },
    }
}

/// here we try to reduce a list of merge marks
/// by checking if they are direct ancestors of each other.
/// specify `is_left_to_right` depending on which direction
/// you want to search through. NOTE that we only
/// check for direct ancestors because thats a O(1) lookup.
/// not sure if it is necessary to traverse the entire
/// ancestor graph?
pub fn filter_ancestor_map(
    filter_state: &mut FilterState,
    merges: &mut Vec<usize>,
    is_left_to_right: bool,
) {
    let len = merges.len();
    match len {
        0 => return,
        1 => return,
        _ => {}
    }

    if is_left_to_right {
        let mut new_merges = vec![merges[0]];
        for m in merges.iter().skip(1) {
            // only add this to the list of actual
            // merges we wish to use, if it is not
            // a direct ancestor of something before it
            if ! filter_state.is_ancestor_of_any(*m, &new_merges) {
                new_merges.push(*m);
            }
        }
        *merges = new_merges;
    } else {
        let mut new_merges = vec![merges[len - 1]];
        for m in merges.iter().rev().skip(1) {
            // only add this to the list of actual
            // merges we wish to use, if it is not
            // a direct ancestor of something before it
            if ! filter_state.is_ancestor_of_any(*m, &new_merges) {
                new_merges.push(*m);
            }
        }
        *merges = new_merges;
    }
}

/// returns a vec of everything that these merge marks
/// point to. three options here:
/// 1. none of the merges exist in the mark map,
/// so basically our parents got filtered out, returns empty vec
/// 2. our 2 or more merges gets filtered to just 1 marked map,
/// which basically means we were a merge commit, but now we
/// only have one parent, so we are now a regular commit.
/// 3. our 2 or more merges get mapped to their correct parents
/// and we are still a merge commit, just potentially with different
/// parents
pub fn resolve_merges(
    filter_state: &mut FilterState,
    merges: &[usize]
) -> Result<Vec<usize>, FilterError> {
    let mut out = vec![];
    for m in merges {
        let mapped_to = get_mapped_mark(filter_state, *m)?;
        // only consider it valid if it doesnt point to empty
        if mapped_to != MAPS_TO_EMPTY {
            out.push(mapped_to);
        }
    }

    out.dedup();

    // // now it gets tricky because
    // // its possible that there are duplicated, or
    // // that one of these just points to the other...
    // // basically we want to prevent:
    // // 1. merge commits that look like this:
    
    // TODO: actually this might be useful.
    // eliminates a lot of trivial dumb merges that
    // we dont need... but need to evaluate
    // if its worth the cost...
    filter_ancestor_map(filter_state, &mut out, true);
    filter_ancestor_map(filter_state, &mut out, false);

    Ok(out)
}

pub fn parent_has_same_contents(
    filter_state: &mut FilterState,
    parent: usize,
    contents: &Vec<FileOpsOwned>,
) -> Result<bool, FilterError> {
    let answer = filter_state.contents_are_same_as(parent, contents)
        .ok_or_else(|| {
            let err_str = format!("Failed to find parent {} while evaluating contents", parent);
            FilterError(err_str)
        })?;
    Ok(answer)
}

/// we expect the commit.fileops to have already
/// applied the filter rules, so we have
/// to check for if we even want this commit
/// ie: it was filtered out, and respond appropriately
pub fn perform_filter2_for_initial_commit(
    filter_state: &mut FilterState,
    commit: &mut StructuredCommit,
) -> Result<FilterResponse, FilterError> {
    // first check if this commits contents were
    // filtered out:
    if commit.fileops.is_empty() {
        // we are an initial commit, and dont have
        // a parent, and we were filtered out. This means
        // we should update the filter map
        // to say that if anyone points to our mark, they are actually
        // pointing to 0, and that means they should remove their from.
        // We have to EXPLICITLY state that otherwise if they search
        // the filter map and dont find an entry, then thats an error.
        // filter_state.
        filter_state.set_mark_map(commit.mark, MAPS_TO_EMPTY);
        return Ok(FilterResponse::DontUse);
    }

    // otherwise we DO want to use this commit.
    // because we are an initial commit, we dont have to worry
    // about froms/merges.. we are good to go!
    // make sure to update mark map to let future
    // commits know that this commit exists and is used!
    filter_state.set_mark_map(commit.mark, commit.mark);
    filter_state.using_commit_with_contents(commit.mark, &commit.merges, &commit.fileops);
    filter_state.update_graph(commit.mark, &commit.merges);
    Ok(FilterResponse::UseAsIs)
}

/// we expect the commit.fileops to have already
/// applied the filter rules, so we have
/// to check for if we even want this commit
/// ie: it was filtered out, and respond appropriately
pub fn perform_filter2_for_regular_commit(
    filter_state: &mut FilterState,
    commit: &mut StructuredCommit,
    parent: usize,
) -> Result<FilterResponse, FilterError> {
    // our parent might not have been included, so we have
    // to check what our parent points to. If our parent WAS included
    // then resolved_parent == parent
    let resolved_parent = get_mapped_mark(filter_state, parent)?;

    // we first check if we are filtered.
    if commit.fileops.is_empty() {
        // we were filtered out, so we have to notify future
        // commits that they should use OUR parent instead of us.
        filter_state.set_mark_map(commit.mark, resolved_parent);
        return Ok(FilterResponse::DontUse);
    }

    // before we check if our parent has same contents
    // as us, check if our parent maps to empty. if so, then
    // we are an initial commit, and we dont have to check
    // our parents contents (because they dont exist)
    if resolved_parent == MAPS_TO_EMPTY {
        // treat ourselves as an initial commit:
        commit.merges = vec![];
        return perform_filter2_for_initial_commit(filter_state, commit);
    }

    if parent_has_same_contents(filter_state, resolved_parent, &commit.fileops)? {
        // we filter ourselves out because we are exactly
        // the same as the parent...
        filter_state.set_mark_map(commit.mark, resolved_parent);
        return Ok(FilterResponse::DontUse);
    }

    // now we are ready to be used, but we have to update
    // ourselves depending on what our parent is actually pointing to:
    if resolved_parent != parent {
        // our parent maps to something else, so update our merges vec
        commit.merges = vec![resolved_parent];
    }

    // Ok we are ready to be used!
    // make sure to let future commits know that we exist!
    filter_state.set_mark_map(commit.mark, commit.mark);
    // also we have to update the graph so that future commits
    // can track parent/child relationships
    filter_state.update_graph(commit.mark, &commit.merges);
    filter_state.using_commit_with_contents(commit.mark, &commit.merges, &commit.fileops);

    return Ok(FilterResponse::UseAsIs)
}

/// we expect the commit.fileops to have already
/// applied the filter rules, so we have
/// to check for if we even want this commit
/// ie: it was filtered out, and respond appropriately
pub fn perform_filter2_for_merge_commit(
    filter_state: &mut FilterState,
    commit: &mut StructuredCommit,
) -> Result<FilterResponse, FilterError> {
    // resolve our parents to one of 3 possible states:
    // 1. still a merge commit, (resolves to 2 or more parents)
    // 2. merge commit became a regular commit (resolves to single parent)
    // 3. merge commit becomes empty (resolves to 0 parents)
    let resolved = resolve_merges(filter_state, &commit.merges)?;
    commit.merges = resolved;
    let new_merges_len = commit.merges.len();
    match new_merges_len {
        0 => {
            // we are now an initial commit, ie: no parents
            return perform_filter2_for_initial_commit(filter_state, commit);
        },
        1 => {
            // cause of slowness.. we already looked up the mark map
            // but then this next function will check again!
            // TODO: add 'already_mapped' flag to this function
            // to avoid trying to map again
            // also TODO: need to check if this
            // fileops is the same as the fileops of its parent...
            // otherwise its just a pointless merge commit that became
            // a regular commit that does the same thing as its parent.
            let parent = commit.merges[0];
            return perform_filter2_for_regular_commit(filter_state, commit, parent);
        },
        _ => {},
    }

    // otherwise we KNOW that we have 2 parents.
    // TODO: how do we filter out empty merge commits?
    // maybe need something like 'parent_has_same_contents' but
    // for all merges?

    // TODO: how to get this to be in the correct order
    // in the first place without explicitly reversing?
    commit.merges.reverse();

    // notify future commits that this mark is valid,
    // and update the graph
    filter_state.set_mark_map(commit.mark, commit.mark);
    filter_state.update_graph(commit.mark, &commit.merges);
    filter_state.using_commit_with_contents(commit.mark, &commit.merges, &commit.fileops);
    Ok(FilterResponse::UseAsIs)
}

pub fn perform_filter2(
    default_include: bool,
    filter_state: &mut FilterState,
    commit: &mut StructuredCommit,
    filter_rules: &FilterRules,
) -> Result<FilterResponse, FilterError> {
    let newfileops = apply_filter_rules_to_fileops(
        default_include, filter_state, commit, filter_rules);
    commit.fileops = newfileops;

    let resp = match commit.merges.len() {
        // this is an initial commit, doesnt have a from line
        // note that it doesnt necessarily have to be the very first commit.
        // just some initial commit in a sequence
        0 => perform_filter2_for_initial_commit(filter_state, commit),
        // regular commit with 1 parent
        1 => {
            let parent = commit.merges[0];
            perform_filter2_for_regular_commit(filter_state, commit, parent)
        },
        // two or more parents: merge commit
        _ => perform_filter2_for_merge_commit(filter_state, commit)
    };
    resp
}

pub fn filter_with_rules<P: AsRef<Path>, T: Write>(
    filter_options: FilterOptions<T>,
    filter_rules: FilterRules,
    location: Option<P>,
) -> io::Result<()> {
    // eprintln!("Filter rules: {:#?}", filter_rules);
    let mut filter_state = FilterState::default();
    let default_include = filter_options.default_include;
    let cb = |obj: &mut StructuredExportObject| -> io::Result<bool> {
        // TODO: filter on blobs as well:
        match &mut obj.object_type {
            export_parser::StructuredObjectType::Blob(_) => Ok(true),
            export_parser::StructuredObjectType::Commit(ref mut c) => {
                let resp = perform_filter2(default_include, &mut filter_state, c, &filter_rules)?;
                if !filter_state.have_used_a_commit && resp.is_used() {
                    filter_state.have_used_a_commit = true;
                }
                let is_used = resp.is_used();
                if let Some(reset) = resp.is_a_reset() {
                    match reset {
                        FilterAsReset::AsReset(resetref) => {
                            obj.has_reset = Some(resetref);
                        }
                        FilterAsReset::AsResetFrom(resetref, resetfrom) => {
                            obj.has_reset = Some(resetref);
                            obj.has_reset_from = Some(resetfrom);
                        }
                    }
                    obj.object_type = export_parser::StructuredObjectType::NoType;
                }
                Ok(is_used)
            },
            _ => Ok(true),
        }
    };
    filter_with_cb(filter_options, location, cb)
}

// temporary function to test out filtering
pub fn filter_with_cb<P: AsRef<Path>, T: Write, F: Into<FilterOptions<T>>>(
    options: F,
    location: Option<P>,
    cb: impl FnMut(&mut StructuredExportObject) -> io::Result<bool>,
) -> io::Result<()> {
    let options: FilterOptions<T> = options.into();
    let mut stream = options.stream;
    let mut cb = cb;
    export_parser::parse_git_filter_export_via_channel(
        options.branch, options.with_blobs, None, location,
        |mut obj| {
            let succeeded = cb(&mut obj)?;
            if succeeded {
                return export_parser::write_to_stream(&mut stream, obj);
            }
            Ok(())
        }
    )?;

    stream.write_all(b"done\n")?;

    Ok(())
}

pub fn filter_with_rules_direct_ex<P: AsRef<Path>, T: Write>(
    filter_options: FilterOptions<T>,
    filter_rules: FilterRules,
    location: Option<P>,
) -> io::Result<()> {
    let exe_and_args = [
        "git", "-c", "core.ignorecase=false", "fast-import", "--date-format=raw-permissive", "--force", "--quiet"
    ];
    let location_clone = match location {
        Some(ref l) => Some(l.as_ref().to_owned()),
        None => None,
    };
    let mut gitimport_handle = exechelper::spawn_with_env_ex2(
        &exe_and_args,
        &[], &[],
        location_clone,
        Some(Stdio::piped()),
        Some(Stdio::null()),
        Some(Stdio::null())
    )?;

    let gitimport_stdin = gitimport_handle.stdin.as_mut().ok_or_else(|| std::io::ErrorKind::InvalidInput)?;
    let overwritten_options = FilterOptions {
        stream: gitimport_stdin,
        branch: filter_options.branch,
        default_include: filter_options.default_include,
        with_blobs: filter_options.with_blobs,
    };

    let res = filter_with_rules(overwritten_options, filter_rules, location);
    let res2 = gitimport_handle.wait();
    if res.is_ok() && res2.is_ok() {
        return Ok(());
    }

    if let Err(e) = res2 {
        Err(e)
    } else {
        res
    }
}

/// filter from your given rules and options, and pipe directly
/// into git fast-import with a sensible default
/// this WILL rewrite your repository history
/// for the branch you provide, and is not reversible.
/// note this uses its own stream, and ignores whatever stream you have
/// in filter_options
pub fn filter_with_rules_direct<T: Write>(
    filter_options: FilterOptions<T>,
    filter_rules: FilterRules,
) -> io::Result<()> {
    let no_location: Option<PathBuf> = None;
    filter_with_rules_direct_ex(filter_options, filter_rules, no_location)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::sink;
    use std::io::Cursor;
    use std::{path::PathBuf, io::Read};
    use export_parser::StructuredObjectType;
    pub const NO_LOCATION: Option<PathBuf> = None;

    #[test]
    fn filter_path_works() {
        let writer = sink();
        filter_with_cb(writer, NO_LOCATION, |obj| {
            match &obj.object_type {
                StructuredObjectType::Blob(_) => Ok(true),
                StructuredObjectType::Commit(commit_obj) => {
                    if commit_obj.committer.email.contains("jerry") {
                        Ok(false)
                    } else {
                        Ok(true)
                    }
                },
                _ => Ok(true),
            }
        }).unwrap();
    }

    #[test]
    fn can_modify_filter_objects() {
        let mut writer = Cursor::new(vec![]);
        filter_with_cb(&mut writer, NO_LOCATION, |obj| {
            if let Some(reset) = &mut obj.has_reset {
                *reset = "refs/heads/NEWBRANCHNAMEAAAAAAA".into();
            }
            match &mut obj.object_type {
                StructuredObjectType::Blob(_) => Ok(true),
                StructuredObjectType::Commit(commit_obj) => {
                    commit_obj.commit_ref = commit_obj.commit_ref.replace("master", "NEWBRANCHNAMEAAAAAAA");
                    Ok(true)
                },
                _ => Ok(true),
            }
        }).unwrap();

        let mut s = String::from("");
        writer.set_position(0);
        writer.read_to_string(&mut s).unwrap();
        assert!(s.contains("refs/heads/NEWBRANCHNAMEAAAAAAA"));
        assert!(!s.contains("refs/heads/master"));
    }

    // used for tests to easily say:
    // construct a commit from these arbitrary file paths
    fn current_commit_state(files: &[&str]) -> StructuredCommit {
        let mut commit = StructuredCommit::default();
        let mut fileops = vec![];
        for file in files {
            let fileop = FileOpsOwned::FileModify(
                "".into(), "".into(), file.to_string(),
            );
            fileops.push(fileop);
        }
        commit.fileops = fileops;
        commit
    }

    #[test]
    fn filter_rules_handle_renames_properly() {
        let mut commit = current_commit_state(&[
            "a.txt"
        ]);
        let mut filter_state = FilterState::default();
        let filter_rule = FilterRule::FilterRulePathRename("a.txt".into(), "b.txt".into());
        let filter_rules = vec![filter_rule];

        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected = FileOpsOwned::FileModify(
            "".into(), "".into(), "b.txt".into(),
        );
        let expected = vec![expected];
        eprintln!("Actual: {:#?}", new_fileops);
        eprintln!("Expected: {:#?}", expected);
        assert_eq!(new_fileops, expected);
    }

    #[test]
    fn filter_rules_handle_rename_to_root_properly() {
        let mut commit = current_commit_state(&[
            "src/a.txt", "src/b.txt"
        ]);
        let mut filter_state = FilterState::default();
        let filter_rule = FilterRule::FilterRulePathRename("src/".into(), "".into());
        let filter_rules = vec![filter_rule];

        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected1 = FileOpsOwned::FileModify(
            "".into(), "".into(), "a.txt".into(),
        );
        let expected2 = FileOpsOwned::FileModify(
            "".into(), "".into(), "b.txt".into(),
        );
        let expected = vec![expected1, expected2];
        eprintln!("Actual: {:#?}", new_fileops);
        eprintln!("Expected: {:#?}", expected);
        assert_eq!(new_fileops, expected);
    }

    #[test]
    fn filter_rules_handle_spaces() {
        let mut commit = current_commit_state(&[
            "\"my folder/a.txt\"", "\"my folder/b.txt\""
        ]);
        let mut filter_state = FilterState::default();
        let filter_rule = FilterRule::FilterRulePathRename("my folder/".into(), "nospace/".into());
        let filter_rules = vec![filter_rule];

        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected1 = FileOpsOwned::FileModify(
            "".into(), "".into(), "nospace/a.txt".into(),
        );
        let expected2 = FileOpsOwned::FileModify(
            "".into(), "".into(), "nospace/b.txt".into(),
        );
        let expected = vec![expected1, expected2];
        eprintln!("Actual: {:#?}", new_fileops);
        eprintln!("Expected: {:#?}", expected);
        assert_eq!(new_fileops, expected);
    }

    #[test]
    fn filter_rules_handle_spaces2() {
        let mut commit = current_commit_state(&[
            "\"my folder/a.txt\"", "\"my folder/b.txt\""
        ]);
        let mut filter_state = FilterState::default();
        let filter_rule = FilterRule::FilterRulePathRename("my folder/".into(), "with space/".into());
        let filter_rules = vec![filter_rule];

        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected1 = FileOpsOwned::FileModify(
            "".into(), "".into(), "\"with space/a.txt\"".into(),
        );
        let expected2 = FileOpsOwned::FileModify(
            "".into(), "".into(), "\"with space/b.txt\"".into(),
        );
        let expected = vec![expected1, expected2];
        eprintln!("Actual: {:#?}", new_fileops);
        eprintln!("Expected: {:#?}", expected);
        assert_eq!(new_fileops, expected);
    }

    #[test]
    fn filter_rules_handle_include_exclude() {
        // we have
        // src/a/
        // src/b/
        // and we want to do:
        // include src/ but exclude src/b/
        // so if we specify filterrules of:
        // FilterRulePathInclude src/
        // FilterRulePathExclude src/b/
        // it should work, but not if we specify it in the other order
        // because the order of the filter rules matters
        // NOTE: if we do FilterRulePathExclude "src/b/"
        // then the second part will work the same as the first, which is only
        // possible because of our exact path exclude matching. in most cases that
        // is what you want, but by default we just check .starts_with()
        // so if we want to match anything that starts with "src/b" (without trailing slash)
        // then this test case is useful because we want to prove that
        // it handles the order correctly
        let mut filter_state = FilterState::default();
        let mut commit = current_commit_state(&[
            "src/a/", "src/b/"
        ]);
        let filter_rule1 = FilterRule::FilterRulePathInclude("src/".into());
        let filter_rule2 = FilterRule::FilterRulePathExclude("src/b".into());
        let filter_rules = vec![filter_rule1.clone(), filter_rule2.clone()];
        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected = FileOpsOwned::FileModify(
            "".into(), "".into(), "src/a/".into(),
        );
        let expected = vec![expected];
        eprintln!("Actual: {:#?}", new_fileops);
        eprintln!("Expected: {:#?}", expected);
        assert_eq!(new_fileops, expected);

        // now do the same thing but reverse the order of
        // the filter rules. it should NOT work:
        let mut filter_state = FilterState::default();
        let mut commit = current_commit_state(&[
            "src/a/", "src/b/"
        ]);
        let filter_rule1 = FilterRule::FilterRulePathInclude("src/".into());
        let filter_rule2 = FilterRule::FilterRulePathExclude("src/b".into());
        let filter_rules = vec![filter_rule2, filter_rule1];
        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected1 = FileOpsOwned::FileModify(
            "".into(), "".into(), "src/a/".into(),
        );
        let expected2 = FileOpsOwned::FileModify(
            "".into(), "".into(), "src/b/".into(),
        );
        let expected = vec![expected1, expected2];
        eprintln!("Actual: {:#?}", new_fileops);
        eprintln!("Expected: {:#?}", expected);
        assert_eq!(new_fileops, expected);

        // just for fun: lets also test that our exact match works:
        // the difference is we say explicitly to exclude src/b/
        // which means it doesnt matter where that rule appears, as
        // soon as we match that rule exactly, we return
        let mut filter_state = FilterState::default();
        let mut commit = current_commit_state(&[
            "src/a/", "src/b/"
        ]);
        let filter_rule1 = FilterRule::FilterRulePathInclude("src/".into());
        let filter_rule2 = FilterRule::FilterRulePathExclude("src/b/".into());
        let filter_rules = vec![filter_rule2, filter_rule1];
        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected = vec![FileOpsOwned::FileModify(
            "".into(), "".into(), "src/a/".into(),
        )];
        eprintln!("Actual: {:#?}", new_fileops);
        eprintln!("Expected: {:#?}", expected);
        assert_eq!(new_fileops, expected);
    }

    #[test]
    fn filter_rules_correct_order() {
        let mut filter_state = FilterState::default();
        let mut commit = current_commit_state(&[
            "lib/src/a.txt",
            "lib/src/a.b",
            "lib/src/a.b.c",
            "lib/src/xyz/hello.txt",
            "lib/src/xyz/something.txt",
        ]);
        let filter_rule1 = FilterRule::FilterRulePathRename("lib/src/".into(), "".into());
        let filter_rule2 = FilterRule::FilterRulePathExclude("lib/src/a.b".into());
        let filter_rule3 = FilterRule::FilterRulePathRename("lib/src/a.b.c".into(), "a.q".into());
        let filter_rule4 = FilterRule::FilterRulePathExclude("lib/src/xyz/something.txt".into());
        let filter_rules = vec![filter_rule1, filter_rule2, filter_rule3, filter_rule4];
        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected = vec![
            "a.txt", "a.q", "xyz/hello.txt"
        ];
        let mut expected_fileops = vec![];
        for path in expected {
            expected_fileops.push(
                FileOpsOwned::FileModify("".into(), "".into(), path.to_string())
            );
        }
        assert_eq!(new_fileops, expected_fileops);
    }

    #[test]
    fn filter_rules_correct_order2() {
        // same as the last one but with a different path rename case
        // where we rename a.b into a.q
        // but want to exclude a.b.c
        let mut filter_state = FilterState::default();
        let mut commit = current_commit_state(&[
            "lib/src/a.txt",
            "lib/src/a.b",
            "lib/src/a.b.c",
            "lib/src/xyz/hello.txt",
            "lib/src/xyz/something.txt",
        ]);
        let filter_rule1 = FilterRule::FilterRulePathRename("lib/src/".into(), "".into());
        let filter_rule2 = FilterRule::FilterRulePathExclude("lib/src/a.b.c".into());
        let filter_rule3 = FilterRule::FilterRulePathRename("lib/src/a.b".into(), "a.q".into());
        let filter_rule4 = FilterRule::FilterRulePathExclude("lib/src/xyz/something.txt".into());
        let filter_rules = vec![filter_rule1, filter_rule2, filter_rule3, filter_rule4];
        let new_fileops = apply_filter_rules_to_fileops(
            false,
            &mut filter_state,
            &mut commit,
            &filter_rules
        );

        let expected = vec![
            "a.txt", "a.q", "xyz/hello.txt"
        ];
        let mut expected_fileops = vec![];
        for path in expected {
            expected_fileops.push(
                FileOpsOwned::FileModify("".into(), "".into(), path.to_string())
            );
        }
        assert_eq!(new_fileops, expected_fileops);
    }
}
