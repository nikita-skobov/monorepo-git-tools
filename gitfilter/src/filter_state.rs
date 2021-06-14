use std::collections::HashMap;
use crate::export_parser::FileOpsOwned;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const MAPS_TO_EMPTY: usize = 0;
pub const UNKNOWN_MAP: usize = usize::MAX;

#[derive(Debug, Default)]
pub struct FilterState {
    pub have_used_a_commit: bool,
    pub graph: Vec<Vec<usize>>,
    pub mark_map: Vec<usize>,
    pub contents_hash_map: HashMap<usize, HashMap<u64, u64>>,
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl FilterState {
    pub fn using_commit_with_contents(
        &mut self,
        mark: usize,
        parents: &[usize],
        contents: &Vec<FileOpsOwned>,
    ) {
        // merge all of the parents hashmaps
        // into a new one.
        // then apply the hashmap from the current contents,
        // and override the parents
        // then insert this final up to date hashmap into
        // the index of this mark.
        let mut parents_merged_map = HashMap::new();
        for p in parents.iter().rev() {
            match self.contents_hash_map.get(p) {
                None => {}
                Some(parent_map) => {
                    for (key, value) in parent_map {
                        parents_merged_map.insert(*key, *value);
                    }
                }
            }
        }        
        for fileop in contents {
            let hash_key_str = match &fileop {
                FileOpsOwned::FileModify(_, _, p) => p,
                FileOpsOwned::FileDelete(p) => p,
                FileOpsOwned::FileCopy(_, p) => p,
                FileOpsOwned::FileRename(_, p) => p,
                FileOpsOwned::FileDeleteAll => "",
                FileOpsOwned::NoteModify(_, p) => p,
            }; 
            let hash_key = calculate_hash(&hash_key_str);
            let hash_value = calculate_hash(fileop);
            parents_merged_map.insert(hash_key, hash_value);
        }
        self.contents_hash_map.insert(mark, parents_merged_map);
    }

    pub fn contents_are_same_as(&self, parent: usize, contents: &Vec<FileOpsOwned>) -> Option<bool> {
        match self.contents_hash_map.get(&parent) {
            Some(parent_hash_map) => {
                // check if every one of our
                // fileops exists in the parent map,
                // and if the hashes are the same:
                let mut every_fileop_exists = true;

                for fileop in contents {
                    let hash_key = match &fileop {
                        FileOpsOwned::FileModify(_, _, p) => p,
                        FileOpsOwned::FileDelete(p) => p,
                        FileOpsOwned::FileCopy(_, p) => p,
                        FileOpsOwned::FileRename(_, p) => p,
                        FileOpsOwned::FileDeleteAll => "",
                        FileOpsOwned::NoteModify(_, p) => p,
                    };
                    let hash_key = calculate_hash(&hash_key);
                    match parent_hash_map.get(&hash_key) {
                        Some(parent_hash_value) => {
                            let hash_value = calculate_hash(fileop);
                            if *parent_hash_value != hash_value {
                                every_fileop_exists = false;
                                break;
                            }
                        }
                        None => {
                            every_fileop_exists = false;
                            break;
                        }
                    }
                }

                Some(every_fileop_exists)
            },
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

    pub fn extend_ancestry_graph_until(&mut self, mark: usize) {
        let len = self.graph.len();
        let desired_len = mark + 1;
        for _ in len..desired_len {
            self.graph.push(vec![]);
        }
    }

    pub fn set_mark_map(&mut self, mark: usize, map: usize) {
        self.extend_mark_map_until(mark);
        self.mark_map[mark] = map;
    }

    pub fn update_graph(&mut self, mark: usize, parents: &[usize]) {
        // self.graph.insert(mark, parents);
        self.extend_ancestry_graph_until(mark);

        // add mark to every single one of the parents
        // ancestry list. and add it so its
        // sorted within that list.
        let mut our_ancestry_table = vec![];
        let mut parents_sorted = parents.to_vec();
        parents_sorted.sort();
        for p in parents_sorted {
            let parents_ancestry_table = &self.graph[p];
            our_ancestry_table.extend(parents_ancestry_table);
            our_ancestry_table.push(p);
        }
        our_ancestry_table.sort();
        our_ancestry_table.dedup();
        self.graph[mark] = our_ancestry_table;
    }

    pub fn get_mapped_mark(&self, mark: usize) -> Option<&usize> {
        self.mark_map.get(mark)
    }

    pub fn is_ancestor(&self, mark: usize, parent: usize) -> bool {
        if mark == parent { return true }

        let parents_ancestry_table = &self.graph[parent];
        for ancestor in parents_ancestry_table {
            if *ancestor == mark {
                return true;
            } else if *ancestor > mark {
                return false;
            }
        }

        false
    }

    pub fn is_ancestor_of_any(&self, mark: usize, parents: &[usize]) -> bool {
        let mut mark_exists_in_a_parent = false;
        for p in parents {
            if self.is_ancestor(mark, *p) {
                mark_exists_in_a_parent = true;
                break;
            }
        }
        mark_exists_in_a_parent
    }
}
