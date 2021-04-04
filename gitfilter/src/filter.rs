use super::export_parser;
use export_parser::{CommitObject, StructuredExportObject, StructuredCommit};
use export_parser::FileOpsOwned;
use super::filter_state::FilterState;
use std::io::Write;
use std::io;

pub enum FilterRule {
    FilterRulePathInclude(String),
    FilterRulePathExclude(String),
}
pub use FilterRule::*;

pub type FilterRules = Vec<FilterRule>;

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

pub fn should_use_file_modify(
    mode: &mut String,
    dataref: &mut String,
    path: &mut String,
    filter_rules: &FilterRules,
    default_include: bool,
) -> bool {
    let mut should_keep = default_include;
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
    default_include: bool,
) -> bool {
    let mut should_keep = default_include;
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
    default_include: bool,
    filter_state: &mut FilterState,
    commit: &mut StructuredCommit,
    filter_rules: &FilterRules,
) -> FilterResponse {
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
                if should_use_file_modify(&mut mode, &mut dataref, &mut path, filter_rules, default_include) {
                    newfileops.push(FileOpsOwned::FileModify(mode, dataref, path));
                }
            }
            FileOpsOwned::FileDelete(mut path) => {
                if should_use_file_delete(&mut path, filter_rules, default_include) {
                    newfileops.push(FileOpsOwned::FileDelete(path));
                }
            }
        }
    }
    // if we have pruned all of the file operations,
    // then we dont want to use this object as a commit, but rather
    // as a reset. Also, make sure to update the mark map with our parent
    // so that if a future commit tries to do:
    // from :THIS
    // then they will instead do:
    // from :THIS_PARENT
    if newfileops.is_empty() {
        match (&commit.from, &commit.mark) {
            (Some(from), Some(mark)) => {
                match filter_state.mark_map.get(from) {
                    Some(transitive_parent) => {
                        // eprintln!("A {} -> {}", mark, transitive_parent);
                        filter_state.mark_map.insert(mark.clone(), transitive_parent.clone());
                    }
                    None => {
                        // eprintln!("B {} -> {}", mark, "");
                        filter_state.mark_map.insert(mark.clone(), "".into());
                    }
                }
            },
            (None, Some(mark)) => {
                // eprintln!("D {:?} -> {:?}", commit.mark, commit.from);
                filter_state.mark_map.insert(mark.clone(), "".into());
            },
            // im not sure these other cases
            // are possible?
            _ => {},
        }
        if commit.merges.is_empty() {
            return FilterResponse::DontUse;
        }
    }
    commit.fileops = newfileops;

    // if this merge doesnt pertain to anything we know about
    // just skip it, dont bother entering it in
    if !commit.merges.is_empty() {
        let has_from = match &commit.from {
            Some(from) => match filter_state.mark_map.get(from) {
                Some(from_points_to) => !from_points_to.is_empty(),
                None => false,
            }
            None => false,
        };
        let has_all_merges = commit.merges.iter().all(|m| {
            match filter_state.mark_map.get(m) {
                Some(pointsto) => !pointsto.is_empty(),
                None => false,
            }
        });
        if !has_from || !has_all_merges {
            return FilterResponse::DontUse;
        }
    }

    // at this point, we know that we will use this commit, so
    // it should map to itself. ie: if we are X, and
    // future commits say
    // from :X
    // then we want it to say from :X, and not
    // from :Parent_of_X
    match &commit.mark {
        Some(mark) => {
            // eprintln!("C {} -> {}", mark, mark);
            filter_state.mark_map.insert(mark.clone(), mark.clone());
        }
        _ => {},
    }
    // if we havent used a commit yet, but this is our first,
    // then we want this to not have a from line:
    if !filter_state.have_used_a_commit {
        commit.from = None;
    }
    // if we are X, and we depend on Z, we should
    // check if Z points to something else.
    // as mentioned above, if Z was filtered out, we have a
    // filter_state.mark_map that contains some parent of Z
    let mut from_pruned = false;
    if let Some(ref mut from) = commit.from {
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
                let panic_str = format!(
                    "Found a commit that we dont know in the map!\nWe are {:?} -> from {}. failed to find the from",
                    commit.mark, from
                );
                panic!(panic_str);
            }
        }
    }

    // if we have a merge commit, its possible it will become not amerge
    // commit (ie: we prune parents from N down to 1)
    // in that case, we want to keep all of their current parents
    // and then traverse them later to find the one that we
    // should map to instead
    let is_merge_commit = commit.from.is_some() && !commit.merges.is_empty();
    let old_merge_points_to = if is_merge_commit {
        let mut pointers = vec![];
        // if the from was pruned, no point in keeping
        // track of it as a parent...
        if !from_pruned {
            if let Some(from) = &commit.from {
                pointers.push(from.clone());
            }
        }
        // but we do want to potentially keep track
        // of the merges... these will be checked below
        for merge in &commit.merges {
            pointers.push(merge.clone());
        }
        Some(pointers)
    } else { None };

    if from_pruned {
        commit.from = None;
        commit.merges = vec![];
    }
    // we also want to do the same thing we did above for the from lines
    // for every single merge. ie: map the original
    // merge :X
    // to say:
    // merge :SOME_PARENT_OF_X
    for merge in &mut commit.merges {
        match filter_state.mark_map.get_mut(merge) {
            Some(mapto) => {
                *merge = mapto.clone();
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
        if commit.merges.is_empty() && commit.from.is_none() {
            // try to add a statement to the map that says
            // we now point to one of our parents. we have to iterate
            // our parents here to find one that still exists.
            if let Some(pointers) = old_merge_points_to {
                if let Some(mark) = &commit.mark {
                    // try the pointers in order of:
                    // FROM, MERGE 1, MERGE 2, ...
                    // and use the first one we find
                    for pointer_option in pointers {
                        // eprintln!("We used to point to: {}", &pointer_option);
                        match filter_state.mark_map.get(&pointer_option) {
                            Some(pointing_to) => {
                                if !pointing_to.is_empty() {
                                    filter_state.mark_map.insert(mark.clone(), pointing_to.clone());
                                    return FilterResponse::DontUse;
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
            // regardless if we were able to find a parent to map to,
            // we still dont want to be used
            return FilterResponse::DontUse;
        }
    }

    FilterResponse::UseAsIs
}

pub fn filter_with_rules<T: Write>(
    filter_options: FilterOptions<T>,
    filter_rules: FilterRules,
) -> io::Result<()> {
    eprintln!("Using branch: {:?}", filter_options.branch);
    let mut filter_state = FilterState::default();
    let default_include = filter_options.default_include;
    let cb = |obj: &mut StructuredExportObject| -> bool {
        // TODO: filter on blobs as well:
        match &mut obj.object_type {
            export_parser::StructuredObjectType::Blob(_) => true,
            export_parser::StructuredObjectType::Commit(ref mut c) => {
                let resp = perform_filter(default_include, &mut filter_state, c, &filter_rules);
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
                is_used
            },
            _ => true,
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
                },
                _ => true,
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
                },
                _ => true,
            }
        }).unwrap();

        let mut s = String::from("");
        writer.set_position(0);
        writer.read_to_string(&mut s).unwrap();
        assert!(s.contains("refs/heads/NEWBRANCHNAMEAAAAAAA"));
        assert!(!s.contains("refs/heads/master"));
    }
}
