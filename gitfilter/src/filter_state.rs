use std::collections::HashMap;
use crate::export_parser::FileOpsOwned;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};


/// would start with 4000 * (4 * 3) = 48000 bytes allocated
pub const MARK_MAP_DEFAULT_CAPACITY: usize = 4000;
pub const MAPS_TO_EMPTY: usize = 0;
pub const MAPS_TO_EMPTY_u32: u32 = 0;
pub const UNKNOWN_MAP: usize = usize::MAX;
pub const UNKNOWN_MAP_u32: u32 = u32::MAX;

/// store a vec of marks. we think of it as a map
/// because our commit mark ids are usize, so we can reference
/// where it exists in the map very quickly without hashing.
/// the first index of marks will be empty so that
/// we don't need to subtract 1 from each mark id (mark ids
/// are never 0).
#[derive(Debug)]
pub struct MarkMap {
    // instead of using usize, we use
    // u32 because we guarantee the size of this...
    // also 32 bits per mark means the maximum number of commits
    // is theorhetically like 4 billion... is that too low?
    // i think not, but we will see.. otherwise we can make
    // this usize, or u64
    // NOTE that we allocate room for 3 items.
    // in most cases the third will be empty.
    // the third element in this array is actually an index
    // (it needs to be casted to usize) into the extras...
    // so if you have a commit with 2 parents, (most merge commits)
    // then the marks entry will look like [A, B, 0]
    // but if you have 3 parents or more, then the third element
    // is the index into the extras vec of where the rest of your parents are
    pub marks: Vec<[u32; 3]>,
    pub extras: Vec<Vec<usize>>,
}

impl MarkMap {
    /// technically should not be necessary because
    /// we visit marks sequentially 1, 2, 3, ...
    /// but just in case, if we get a mark of 10,
    /// and currently our marks vec only has len of 3, then
    /// we need to insert 7 empty marks.
    /// this function should guarantee that after calling this
    /// the length of marks is AT LEAST mark + 1
    /// (because we want to be able to do self.marks[mark])
    pub fn extend_marks_until(&mut self, mark: usize) {
        let desired_len = mark + 1;
        let current_len = self.marks.len();
        for _ in current_len..desired_len {
            // use 0 0 max to indicate not a valid state
            self.marks.push([MAPS_TO_EMPTY_u32, MAPS_TO_EMPTY_u32, UNKNOWN_MAP_u32]);
        }
    }

    /// parents must have at least 1 element!
    pub fn insert(&mut self, mark: usize, parents: &[usize]) {
        self.extend_marks_until(mark);
        let len = parents.len();
        let mark_segment = &mut self.marks[mark];
        if len == 1 {
            *mark_segment = [parents[0] as u32, MAPS_TO_EMPTY_u32, MAPS_TO_EMPTY_u32];
        } else if len == 2 {
            *mark_segment = [parents[0] as u32, parents[1] as u32, MAPS_TO_EMPTY_u32];
        } else if len > 2 {
            *mark_segment = [parents[0] as u32, parents[1] as u32, MAPS_TO_EMPTY_u32];
            let mut extra = vec![];
            for i in 2..len {
                extra.push(parents[i]);
            }
            self.extras.push(extra);
            let extra_index = self.extras.len() - 1;
            (*mark_segment)[2] = extra_index as u32;
        };
    }

    /// get a vec of marks that are parents of this mark
    pub fn get_all_mapped(&self, mark: usize) -> Option<Vec<usize>> {
        if self.marks.len() < mark + 1 {
            return None;
        }
        let mark_segment = &self.marks[mark];
        // this is an invalid segment, ie: it was never actually set to
        // anything
        if *mark_segment == [MAPS_TO_EMPTY_u32, MAPS_TO_EMPTY_u32, UNKNOWN_MAP_u32] {
            return None;
        }
        let mut out = Vec::with_capacity(2);
        out.push(mark_segment[0] as usize);
        if mark_segment[1] != MAPS_TO_EMPTY_u32 {
            out.push(mark_segment[1] as usize);
        }
        if mark_segment[2] != MAPS_TO_EMPTY_u32 {
            let extras_index = mark_segment[2] as usize;
            for extra in self.extras[extras_index].iter() {
                out.push(*extra);
            }
        }

        Some(out)
    }
}

impl Default for MarkMap {
    fn default() -> Self {
        let mut marks_vec = Vec::with_capacity(MARK_MAP_DEFAULT_CAPACITY);
        // we want it to start with one empty!
        let extras_vec = vec![vec![]];
        marks_vec.push([0, 0, 0]);
        MarkMap {
            marks: marks_vec,
            extras: extras_vec,
        }
    }
}

#[derive(Debug, Default)]
pub struct FilterState {
    pub have_used_a_commit: bool,
    pub graph: MarkMap,
    pub mark_map: Vec<usize>,
    pub contents_hash_map: HashMap<usize, u64>,
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl FilterState {
    pub fn using_commit_with_contents(&mut self, mark: usize, contents: &Vec<FileOpsOwned>) {
        let hash = calculate_hash(contents);
        self.contents_hash_map.insert(mark, hash);
    }

    pub fn contents_are_same_as(&self, parent: usize, contents: &Vec<FileOpsOwned>) -> Option<bool> {
        let hash = calculate_hash(contents);
        match self.contents_hash_map.get(&parent) {
            Some(parent_hash) => Some(*parent_hash == hash),
            None => None
        }
    }

    pub fn extend_mark_map_until(&mut self, mark: usize) {
        let len = self.mark_map.len();
        let desired_len = mark + 1;
        for _ in len..desired_len {
            self.mark_map.push(UNKNOWN_MAP);
        }
    }

    pub fn set_mark_map(&mut self, mark: usize, map: usize) {
        self.extend_mark_map_until(mark);
        self.mark_map[mark] = map;
    }

    pub fn update_graph(&mut self, mark: usize, parents: &[usize]) {
        self.graph.insert(mark, parents);
    }

    pub fn get_mapped_mark(&self, mark: usize) -> Option<&usize> {
        self.mark_map.get(mark)
    }

    /// is mark a direct ancestor of parent, ie:
    /// does mark exist in parent's parents
    pub fn is_direct_ancestor(&self, mark: usize, parent: usize) -> bool {
        if mark == parent { return true }
        match self.graph.get_all_mapped(parent) {
            Some(parents) => parents.contains(&mark),
            None => {
                // this means we failed to find parent in the
                // graph... TODO: is this an error?
                false
            }
        }
    }

    pub fn is_direct_ancestor_of_any(&self, mark: usize, parents: &[usize]) -> bool {
        let mut mark_exists_in_a_parent = false;
        for p in parents {
            if self.is_direct_ancestor(mark, *p) {
                mark_exists_in_a_parent = true;
                break;
            }
        }
        mark_exists_in_a_parent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_map_works() {
        let mut map = MarkMap::default();
        assert_eq!(map.marks.len(), 1);
        assert_eq!(map.extras.len(), 1);

        // if we insert 10, then
        // map len should be 11
        map.insert(10, &[1]);
        assert_eq!(map.marks.len(), 11);
        assert_eq!(map.extras.len(), 1);

        // can insert at index 5 now, and len should be the same
        map.insert(5, &[2, 3]);
        assert_eq!(map.marks.len(), 11);
        assert_eq!(map.extras.len(), 1);

        // finally, let's try adding
        // a mark with more than 2 parents.
        // the extras should grow now
        map.insert(11, &[4, 5, 6]);
        assert_eq!(map.marks.len(), 12);
        assert_eq!(map.extras.len(), 2);

        // now let's try to retrieve all of the parents
        // we just put in:
        let m1 = map.get_all_mapped(10).unwrap();
        assert_eq!(m1, [1]);
        let m2 = map.get_all_mapped(5).unwrap();
        assert_eq!(m2, [2, 3]);
        let m3 = map.get_all_mapped(11).unwrap();
        assert_eq!(m3, [4, 5, 6]);
    }
}
