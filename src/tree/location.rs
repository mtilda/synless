use super::node::Node;
use crate::language::{Arity, Storage};
use crate::util::{bug, SynlessBug};

// The node in this LocationInner may not be valid (may have been deleted!)
#[derive(Debug, Clone, Copy)]
pub struct Bookmark(LocationInner);

/// A location between nodes, or within text, where a cursor could go.
#[derive(Debug, Clone, Copy)]
pub struct Location(LocationInner);

/// This data type admits multiple representations of the same location. For example, a location
/// between nodes X and Y could be represented as either `AfterNode(X)` or `BeforeNode(Y)`. We
/// therefore keep locations in a _normal form_. The exception is Bookmarks, which might not be in
/// normal form (or even valid!) and must be checked and normalized before use. The rule for the
/// normal form is that `AfterNode` is used if possible, falling back to `BeforeNode` and then
/// `BelowNode`. This implies that `BelowNode` is only used in empty sequences.
#[derive(Debug, Clone, Copy)]
enum LocationInner {
    /// The usize is an index between chars (so it can be equal to the len)
    InText(Node, usize),
    AfterNode(Node),
    BeforeNode(Node),
    BelowNode(Node),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Tree,
    Text,
}

impl Location {
    /****************
     * Constructors *
     ****************/

    pub fn before(node: Node, s: &Storage) -> Location {
        Location(LocationInner::BeforeNode(node).normalize(s))
    }

    pub fn after(node: Node, _s: &Storage) -> Location {
        // already normal form
        Location(LocationInner::AfterNode(node))
    }

    /// Returns the location at the beginning of the child sequence of the given node.
    pub fn before_children(node: Node, s: &Storage) -> Option<Location> {
        if node.is_texty(s) {
            return None;
        }
        if let Some(first_child) = node.first_child(s) {
            Some(Location::before(first_child, s))
        } else {
            Some(Location(LocationInner::BelowNode(node)))
        }
    }

    /// Returns the location at the end of the child sequence of the given node.
    pub fn after_children(node: Node, s: &Storage) -> Option<Location> {
        if node.is_texty(s) {
            return None;
        }
        if let Some(last_child) = node.last_child(s) {
            Some(Location::after(last_child, s))
        } else {
            Some(Location(LocationInner::BelowNode(node)))
        }
    }

    /*************
     * Accessors *
     *************/

    pub fn mode(self) -> Mode {
        match self.0 {
            LocationInner::InText(_, _) => Mode::Text,
            _ => Mode::Tree,
        }
    }

    pub fn text_pos(self) -> Option<(Node, usize)> {
        if let LocationInner::InText(node, char_pos) = self.0 {
            Some((node, char_pos))
        } else {
            None
        }
    }

    pub fn text_pos_mut(&mut self) -> Option<(Node, &mut usize)> {
        if let LocationInner::InText(node, char_pos) = &mut self.0 {
            Some((*node, char_pos))
        } else {
            None
        }
    }

    /**************
     * Navigation *
     **************/

    pub fn prev(self, s: &Storage) -> Option<Location> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            InText(_, _) => None,
            AfterNode(node) => Some(Location::before(node, s)),
            BeforeNode(_) => None,
            BelowNode(_) => None,
        }
    }

    pub fn next(self, s: &Storage) -> Option<Location> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            InText(_, _) => None,
            AfterNode(node) => Some(Location::after(node.next_sibling(s)?, s)),
            BeforeNode(node) => Some(Location::after(node, s)),
            BelowNode(_) => None,
        }
    }

    pub fn first(self, s: &Storage) -> Option<Location> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            InText(_, _) => None,
            AfterNode(node) => Some(Location::before(node.first_sibling(s), s)),
            BeforeNode(_) | BelowNode(_) => Some(self),
        }
    }

    pub fn last(self, s: &Storage) -> Option<Location> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            InText(_, _) => None,
            BeforeNode(node) | AfterNode(node) => Some(Location::after(node.last_sibling(s), s)),
            BelowNode(_) => Some(self),
        }
    }

    pub fn before_parent(self, s: &Storage) -> Option<Location> {
        Some(Location::before(self.parent_node(s)?, s))
    }

    pub fn after_parent(self, s: &Storage) -> Option<Location> {
        Some(Location::after(self.parent_node(s)?, s))
    }

    /// Returns the next location in an inorder tree traversal.
    pub fn inorder_next(self, s: &Storage) -> Option<Location> {
        if let Some(right_node) = self.right_node(s) {
            if let Some(loc) = Location::before_children(right_node, s) {
                Some(loc)
            } else {
                Some(Location::after(right_node, s))
            }
        } else {
            self.after_parent(s)
        }
    }

    /// Returns the previous location in an inorder tree traversal.
    pub fn inorder_prev(self, s: &Storage) -> Option<Location> {
        if let Some(left_node) = self.left_node(s) {
            if let Some(loc) = Location::after_children(left_node, s) {
                Some(loc)
            } else {
                Some(Location::before(left_node, s))
            }
        } else {
            self.before_parent(s)
        }
    }

    /// Returns the location at the end of the texty node that is before the current location.
    pub fn enter_text(self, s: &Storage) -> Option<Location> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            AfterNode(node) => {
                let text_len = node.text(s)?.num_chars();
                Some(Location(LocationInner::InText(node, text_len)))
            }
            InText(_, _) | BeforeNode(_) | BelowNode(_) => None,
        }
    }

    /// If the location is in text, returns the location after that text node.
    pub fn exit_text(self) -> Option<Location> {
        if let LocationInner::InText(node, _) = self.0 {
            // already in normal form
            Some(Location(LocationInner::AfterNode(node)))
        } else {
            None
        }
    }

    /**********************
     * Navigation to Node *
     **********************/

    pub fn left_node(self, _s: &Storage) -> Option<Node> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            InText(_, _) => None,
            AfterNode(node) => Some(node),
            BeforeNode(_) => None,
            BelowNode(_) => None,
        }
    }

    pub fn right_node(self, s: &Storage) -> Option<Node> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            InText(_, _) => None,
            AfterNode(node) => node.next_sibling(s),
            BeforeNode(node) => Some(node),
            BelowNode(_) => None,
        }
    }

    pub fn parent_node(self, s: &Storage) -> Option<Node> {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self.0 {
            InText(node, _) => None,
            AfterNode(node) => node.parent(s),
            BeforeNode(node) => node.parent(s),
            BelowNode(node) => Some(node),
        }
    }

    pub fn root_node(self, s: &Storage) -> Node {
        self.0.node().root(s)
    }

    /************
     * Mutation *
     ************/

    #[allow(clippy::result_unit_err)]
    pub fn insert(&mut self, new_node: Node, s: &mut Storage) -> Result<Option<Node>, ()> {
        use LocationInner::*;

        let parent = self.parent_node(s).ok_or(())?;

        match parent.arity(s) {
            Arity::Texty => bug!("insert: texty parent"),
            Arity::Fixed(_) => {
                let old_node = self.right_node(s).ok_or(())?;
                if new_node.swap(s, old_node) {
                    *self = Location::after(new_node, s);
                    Ok(Some(old_node))
                } else {
                    Err(())
                }
            }
            Arity::Listy(_) => {
                let success = match self.0 {
                    InText(_, _) => false,
                    AfterNode(left_node) => left_node.insert_after(s, new_node),
                    BeforeNode(right_node) => right_node.insert_before(s, new_node),
                    BelowNode(_) => parent.insert_last_child(s, new_node),
                };
                if success {
                    *self = Location::after(new_node, s);
                    Ok(None)
                } else {
                    Err(())
                }
            }
        }
    }

    #[must_use]
    pub fn delete_neighbor(&mut self, on_left: bool, s: &mut Storage) -> Option<Node> {
        let parent = self.parent_node(s)?;
        let node = if on_left {
            self.left_node(s)?
        } else {
            self.right_node(s)?
        };
        match parent.arity(s) {
            Arity::Fixed(_) => {
                let hole = Node::new_hole(s, parent.language(s));
                if node.swap(s, hole) {
                    Some(node)
                } else {
                    None
                }
            }
            Arity::Listy(_) => {
                if node.detach(s) {
                    Some(node)
                } else {
                    None
                }
            }
            Arity::Texty => bug!("delete_neighbor: texty parent"),
        }
    }

    /*************
     * Bookmarks *
     *************/

    /// Save a bookmark to return to later.
    pub fn bookmark(self) -> Bookmark {
        Bookmark(self.0)
    }

    /// Get the location of a previously saved bookmark, as long as that
    /// bookmark's node is present somewhere in this tree. This will
    /// work even if the Tree has been modified since the bookmark was
    /// created. However, it will return `None` if the bookmark's node
    /// has since been deleted, or if it is currently located in a
    /// different tree.
    pub fn validate_bookmark(self, mark: Bookmark, s: &Storage) -> Option<Location> {
        let mark_node = mark.0.node();
        if mark_node.is_valid(s) && mark_node.root(s) == self.root_node(s) {
            Some(Location(mark.0.normalize(s)))
        } else {
            None
        }
    }
}

impl LocationInner {
    fn normalize(self, s: &Storage) -> LocationInner {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self {
            InText(node, i) => {
                let text_len = node.text(s).bug().num_chars();
                InText(node, i.min(text_len))
            }
            AfterNode(_) => self,
            BeforeNode(node) => node.prev_sibling(s).map(AfterNode).unwrap_or(self),
            BelowNode(parent) => parent.last_child(s).map(AfterNode).unwrap_or(self),
        }
    }

    fn node(self) -> Node {
        use LocationInner::{AfterNode, BeforeNode, BelowNode, InText};

        match self {
            InText(node, _) | AfterNode(node) | BeforeNode(node) | BelowNode(node) => node,
        }
    }
}
