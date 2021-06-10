use super::export_parser;
use export_parser::{StructuredExportObject, StructuredCommit};
use export_parser::FileOpsOwned;
use super::filter_state::FilterState;
use std::io::Write;
use std::process::Stdio;
use std::{path::{PathBuf, Path}, io};
use io::Error;

#[derive(Clone, Debug)]
pub enum FilterRule {
    FilterRulePathInclude(String),
    FilterRulePathExclude(String),
    FilterRulePathRename(String, String),
}
pub use FilterRule::*;

pub type FilterRules = Vec<FilterRule>;
pub struct FilterError(String);

impl From<FilterError> for io::Error {
    fn from(orig: FilterError) -> Self {
        io::Error::new(io::ErrorKind::Other, orig.0)
    }
}

/// how to use this filtered commit
pub enum FilterResponse {
    /// dont use it, dont output anything, skip it entirely
    DontUse,
    /// either use it as is, or if it was already modified
    /// by the user, then use what the user modified 
    UseAsIs,

    UseAsReset(FilterAsReset),
}

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

pub fn perform_filter(
    default_include: bool,
    filter_state: &mut FilterState,
    commit: &mut StructuredCommit,
    filter_rules: &FilterRules,
) -> Result<FilterResponse, FilterError> {
    // eprintln!("We see:\n{:#?}", commit);
    let newfileops = apply_filter_rules_to_fileops(default_include, filter_state, commit, filter_rules);
    // if we have pruned all of the file operations,
    // then we dont want to use this object as a commit, but rather
    // as a reset. Also, make sure to update the mark map with our parent
    // so that if a future commit tries to do:
    // from :THIS
    // then they will instead do:
    // from :THIS_PARENT
    
    // TODO: this is for temporary compatibility...
    // get rid of this. theres no reason to make this a string
    let mut from = match commit.merges.first() {
        Some(f) => Some(f.to_string()),
        None => None,
    };
    let mark = commit.mark.to_string();

    if newfileops.is_empty() {
        if let Some(from) = from {
            let insert_with = match filter_state.mark_map.get(&from) {
                Some(transitive_parent) => Some(transitive_parent.clone()),
                None => None,
            };
            match insert_with {
                Some(transitive_parent) => {
                    // eprintln!("A {} -> {}", mark, transitive_parent);
                    filter_state.mark_map.insert(mark.clone(), transitive_parent.clone());
                }
                None => {
                    // eprintln!("B {} -> {}", mark, "");
                    filter_state.mark_map.insert(mark.clone(), "".into());
                }
            }
        } else {
            // eprintln!("D {:?} -> {:?}", commit.mark, commit.from);
            filter_state.mark_map.insert(mark.clone(), "".into());
        }
        return Ok(FilterResponse::DontUse);
    }
    commit.fileops = newfileops;

    // if this merge doesnt pertain to anything we know about
    // just skip it, dont bother entering it in
    if !commit.merges.is_empty() {
        let has_from = match &from {
            Some(from) => filter_state.has_nonempty_mark(from),
            None => false,
        };
        // TODO: add a impl fn for the filter state to
        // test if a mark exists and is non emtpy. basically
        // dont repeat this code over and over:
        let has_all_merges = commit.merges.iter()
            .all(|m| filter_state.has_nonempty_mark(&m.to_string()));

        if !has_from && !has_all_merges {
            // eprintln!("Dont use because it doesnt has from and it doesnt have merges");
            return Ok(FilterResponse::DontUse);
        } else if has_from && !has_all_merges {
            // if the from exists, but the merges dont, then
            // remove all merges that dont exist:
            let mut is_fist_merge = true;
            commit.merges.retain(|merge| {
                if is_fist_merge {
                    is_fist_merge = false;
                    true
                } else {
                    filter_state.has_nonempty_mark(&merge.to_string())
                }
            });
        }
    }

    // at this point, we know that we will use this commit, so
    // it should map to itself. ie: if we are X, and
    // future commits say
    // from :X
    // then we want it to say from :X, and not
    // from :Parent_of_X
    // if let Some(mark) = &mark {
        // eprintln!("C {} -> {}", mark, mark);
        filter_state.mark_map.insert(mark.clone(), mark.clone());
    // }

    // if we havent used a commit yet, but this is our first,
    // then we want this to not have a from line:
    if !filter_state.have_used_a_commit {
        from = None;
    }
    // if we are X, and we depend on Z, we should
    // check if Z points to something else.
    // as mentioned above, if Z was filtered out, we have a
    // filter_state.mark_map that contains some parent of Z
    let mut from_pruned = false;
    if let Some(ref mut from) = from {
        match filter_state.mark_map.get(from) {
            Some(mapto) => {
                if mapto.is_empty() {
                    // this indicates we are trying to point to a from
                    // that doesnt actually exist. in this case,
                    // instead of using a from, we want to remove the
                    // from and any merges
                    from_pruned = true;
                } else {
                    *from = mapto.clone();
                }
            }
            None => {
                // TODO: remove panic, and return a result instead
                let err_str = format!(
                    "Found a commit that we dont know in the map!\nWe are {:?} -> from {}. failed to find the from",
                    commit.mark, from
                );
                return Err(FilterError(err_str));
            }
        }
    }

    // if we have a merge commit, its possible it will become not amerge
    // commit (ie: we prune parents from N down to 1)
    // in that case, we want to keep all of their current parents
    // and then traverse them later to find the one that we
    // should map to instead
    let has_merges_other_than_from = commit.merges.len() > 1;
    let is_merge_commit = from.is_some() && has_merges_other_than_from;
    let old_merge_points_to = if is_merge_commit {
        let mut pointers = vec![];
        // if the from was pruned, no point in keeping
        // track of it as a parent...
        if !from_pruned {
            if let Some(from) = &from {
                pointers.push(from.clone());
            }
        }
        // but we do want to potentially keep track
        // of the merges... these will be checked below
        for merge in commit.merges.iter().skip(1) {
            pointers.push(merge.to_string());
        }
        Some(pointers)
    } else { None };

    if from_pruned {
        commit.merges = vec![];
    }
    // we also want to do the same thing we did above for the from lines
    // for every single merge. ie: map the original
    // merge :X
    // to say:
    // merge :SOME_PARENT_OF_X
    for merge in commit.merges.iter_mut().skip(1) {
        match filter_state.mark_map.get_mut(&merge.to_string()) {
            Some(mapto) => {
                *merge = mapto.parse::<usize>().unwrap_or(0);
            }
            None => {}
        }
    }

    // we used to be a merge commit, but because of pruning
    // we became a regular commit. this basically means that
    // the contents of this commit are the same as one of the
    // prior commits we are merging, ie: a duplicate.
    // the decision here is to drop it, but i think optionally,
    // if you want to allow empty merge commits, maybe have some setting
    // that keeps this commit with just the message but no changes
    if is_merge_commit {
        let no_commit_besides_from = commit.merges.len() <= 1;
        if no_commit_besides_from && from.is_none() {
            // try to add a statement to the map that says
            // we now point to one of our parents. we have to iterate
            // our parents here to find one that still exists.
            if let Some(pointers) = &old_merge_points_to {
                // if let Some(mark) = &commit.mark {
                // try the pointers in order of:
                // FROM, MERGE 1, MERGE 2, ...
                // and use the first one we find
                for pointer_option in pointers {
                    // eprintln!("We used to point to: {}", &pointer_option);
                    let pointed_to = match filter_state.mark_map.get(pointer_option) {
                        Some(pointing_to) => {
                            if !pointing_to.is_empty() {
                                Some(pointing_to.clone())
                            } else { None }
                        }
                        None => None,
                    };
                    if let Some(pointing_to) = pointed_to {
                        filter_state.mark_map.insert(mark.clone(), pointing_to);
                        return Ok(FilterResponse::DontUse);
                    }
                }
                // }
            }
            // regardless if we were able to find a parent to map to,
            // we still dont want to be used
            return Ok(FilterResponse::DontUse);
        }
    }

    // if its a merge commit, make sure that we dont have
    // a scenario where:
    //   A _
    //   |  \
    //   X  |
    //   Y /
    // basically if A is a merge of X, and Y,
    // we make sure that X isnt a direct ancestor of Y
    // and if so, we prune A, because thats an unnecessary merge commit
    // it would just be empty.
    if is_merge_commit {
        // TODO: handle octopus merges...
        // right now this will only handle the case where theres a merge
        // with 2 parents:
        if let Some(pointers) = &old_merge_points_to {
            if pointers.len() > 1 {
                let parent_x = &pointers[0];
                let parent_y = &pointers[1];

                let x_is_ancestor_of_y = match filter_state.graph.get(parent_x) {
                    Some(x_parents) => x_parents.contains(parent_y),
                    None => false,
                };
                let y_is_ancestor_of_x = match filter_state.graph.get(parent_y) {
                    Some(y_parents) => y_parents.contains(parent_x),
                    None => false,
                };
                if x_is_ancestor_of_y || y_is_ancestor_of_x {
                    // if let Some(mark) = &commit.mark {
                    // eprintln!("NOT USING {} Because its a merge commit whose ancestors ({:?}) are direct parents of each other. Its mark map is: {:?}", mark, pointers, filter_state.mark_map.get(mark));
                    // remember to update map. we want to then point to the
                    // most recent child, so we check specifically which case we got:
                    if x_is_ancestor_of_y {
                        // in this case we have: THIS_COMMIT -> X -> Y
                        // so we want to say our mark points to X
                        filter_state.mark_map.insert(mark.clone(), parent_x.clone());
                    } else {
                        // otherwise y is ancestor of x
                        // in this case we have: THIS_COMMIT -> Y -> X
                        // so we want to say our mark points to Y
                        filter_state.mark_map.insert(mark.clone(), parent_y.clone());
                    }
                    // }

                    return Ok(FilterResponse::DontUse);
                }
            }
        }
    }

    // update the current graph to say that
    // we are X, and we have parent(s) Y/Z:
    let mut parents = match &from {
        Some(parent1) => vec![parent1.clone()],
        None => vec![]
    };
    for merge in commit.merges.iter().skip(1) {
        parents.push(merge.to_string());
    }
    filter_state.graph.insert(mark.clone(), parents.clone());
    // match &commit.mark {
    //     Some(mark) => {

    //     }
    //     _ => {},
    // }

    Ok(FilterResponse::UseAsIs)
}

pub fn filter_with_rules<P: AsRef<Path>, T: Write>(
    filter_options: FilterOptions<T>,
    filter_rules: FilterRules,
    location: Option<P>,
) -> io::Result<()> {
    let mut filter_state = FilterState::default();
    let default_include = filter_options.default_include;
    let cb = |obj: &mut StructuredExportObject| -> io::Result<bool> {
        // TODO: filter on blobs as well:
        match &mut obj.object_type {
            export_parser::StructuredObjectType::Blob(_) => Ok(true),
            export_parser::StructuredObjectType::Commit(ref mut c) => {
                let resp = perform_filter(default_include, &mut filter_state, c, &filter_rules)?;
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
        options.branch, options.with_blobs, location,
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
