use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct FilterState {
    pub have_used_a_commit: bool,
    pub mark_map: HashMap<String, String>,
    pub graph: HashMap<String, Vec<String>>,
}
