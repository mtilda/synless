use super::{DocStorage, Node};

/// A location between nodes, or within text, where a cursor could go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    InText(Node, usize),
    After(Node),
    BeforeFirstChild(Node),
}

impl Location {
    pub fn cursor_halves(self, s: &DocStorage) -> (Option<Node>, Option<Node>) {
        match self {
            Location::InText(..) => (None, None),
            Location::After(left_sibling) => (Some(left_sibling), left_sibling.next_sibling(s)),
            Location::BeforeFirstChild(parent) => (None, parent.first_child(s)),
        }
    }
}
