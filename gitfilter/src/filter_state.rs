use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct FilterState {
    pub have_used_a_commit: bool,
    pub mark_map: HashMap<usize, usize>,
    pub graph: HashMap<usize, Vec<usize>>,
}

impl FilterState {
    /// returns true of the mark_map has this mark, and its not empty
    pub fn has_nonempty_mark(&self, mark: usize) -> bool {
        match self.mark_map.get(&mark) {
            Some(m) => *m != 0,
            None => false,
        }
    }

    pub fn set_mark_map(&mut self, mark: usize, map: usize) {
        self.mark_map.insert(mark, map);
    }

    pub fn get_mapped_mark(&self, mark: usize) -> Option<&usize> {
        self.mark_map.get(&mark)
    }

    pub fn update_graph(&mut self, mark: usize, parents: &Vec<usize>) {
        self.graph.insert(mark, parents.to_owned());
    }

    /// check if the mark is an ancestor of parent
    pub fn is_ancestor(&self, mark: usize, parent: usize) -> bool {
        match self.graph.get(&parent) {
            Some(parents) => {
                let mark_in_parents = parents.contains(&mark);
                // just because the mark isnt in one of these parents,
                // doesnt mean it doesnt exist somewhere in the parents of these...
                if ! mark_in_parents {
                    // need to iterate all parents and check
                    // if mark is somewhere in there
                    let mut exists_in_a_parent = false;
                    for p in parents {
                        if self.is_ancestor(mark, *p) {
                            exists_in_a_parent = true;
                            break;
                        }
                    }
                    exists_in_a_parent
                } else {
                    // YES, mark is an ancestor of parent, because
                    return true;
                }
            }
            None => false,
        }
    }

    // returns true if mark is an ancestor of ANY of the parents,
    // and false if the mark is NOT an ancestor of any parent
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

    pub fn is_direct_ancestor(&self, mark: usize, parent: usize) -> bool {
        match self.graph.get(&parent) {
            None => false,
            Some(parents) => parents.contains(&mark),
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
