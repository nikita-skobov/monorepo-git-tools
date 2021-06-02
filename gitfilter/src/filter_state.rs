use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct FilterState {
    pub have_used_a_commit: bool,
    pub mark_map: HashMap<String, String>,
    pub graph: HashMap<String, Vec<String>>,
}

impl FilterState {
    /// returns true of the mark_map has this mark, and its not empty
    pub fn has_nonempty_mark(&self, mark: &str) -> bool {
        match self.mark_map.get(mark) {
            Some(m) => !m.is_empty(),
            None => false,
        }
    }
}
