//! A collection of trees.
//!
//! There are two kinds of tree nodes: _branch_ nodes and _leaf_ nodes.
//! Both kinds of nodes have data D. In addition:
//!
//! - Branch nodes have zero or more children
//! - Leaf nodes have _additional_ data L
//!
//! **Every method on `Node` may panic, if that node was deleted.**
//! Deleting the ancestor of a node will delete the node. The one exception
//! is the `is_valid()` method, which checks whether a node has been deleted.
//!
//! This library solves these problems:
//!
//! - Ensuring that parent and child links always agree.
//! - Ensuring that every tree is accounted for.
//! - Preventing cycles at runtime.
//!
//! It does NOT solve these problems:
//!
//! - Preventing "use after free" (see the note on deletion above).
//!   Along the same lines, preventing cycles at compile time.
//! - Removing the need to pass the `Forest` in to every method call.

// Note to self: solving either of the "does not solve" problems will ~double
// the size of this library, and thereby also increase the size of its
// caller because the caller will need to wrap everything. It's not worth it.
// Let the caller deal with it.

// INVARIANTS:
// - n.parent is None iff n in Forest.roots
// - n.parent is Some(p) iff arena[p] is a branch node containing n
// - Every node is a descandant of one of the roots

use generational_arena::{Arena, Index};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Forest<D, L> {
    roots: Vec<Node<D, L>>,
    arena: Arena<NodeContents<D, L>>,
}

#[derive(Debug)]
pub struct Node<D, L>(Index, PhantomData<(D, L)>);

#[derive(Debug, PartialEq, Eq)]
pub struct Bookmark<D, L>(Index, PhantomData<(D, L)>);

#[derive(Debug)]
struct NodeContents<D, L> {
    parent: Option<Node<D, L>>,
    data: D,
    children: NodeChildren<D, L>,
}

#[derive(Debug)]
pub enum NodeChildren<D, L> {
    Leaf(L),
    Branch(Vec<Node<D, L>>),
}

impl<D, L> PartialEq for Node<D, L> {
    fn eq(&self, other: &Node<D, L>) -> bool {
        self.0 == other.0
    }
}

impl<D, L> Eq for Node<D, L> {}

impl<D, L> Clone for Node<D, L> {
    fn clone(&self) -> Node<D, L> {
        Node(self.0, self.1)
    }
}

impl<D, L> Copy for Node<D, L> {}

impl<D, L> Forest<D, L> {
    /// Construct a new forest.
    pub fn new() -> Forest<D, L> {
        Forest {
            roots: Vec::new(),
            arena: Arena::new(),
        }
    }

    /// Construct a new leaf.
    pub fn new_leaf(&mut self, data: D, leaf: L) -> Node<D, L> {
        let node = Node::new(self.arena.insert(NodeContents {
            parent: None,
            data,
            children: NodeChildren::Leaf(leaf),
        }));
        self.roots.push(node);
        node
    }

    /// Construct a new branch with no children.
    pub fn new_branch(&mut self, data: D) -> Node<D, L> {
        let node = Node::new(self.arena.insert(NodeContents {
            parent: None,
            data,
            children: NodeChildren::Branch(Vec::new()),
        }));
        self.roots.push(node);
        node
    }

    /// Iterate over all nodes. This is an ExactSizeIterator, so you can call `.len()`.
    pub fn iter_nodes(&self) -> impl ExactSizeIterator<Item = Node<D, L>> + '_ {
        self.arena.iter().map(|(id, _)| Node::new(id))
    }

    /// All trees.
    pub fn roots(&self) -> &[Node<D, L>] {
        &self.roots
    }

    fn get(&self, node: Node<D, L>) -> &NodeContents<D, L> {
        match self.arena.get(node.0) {
            None => panic!("Forest - node has been deleted!"),
            Some(node) => node,
        }
    }

    fn get_mut(&mut self, node: Node<D, L>) -> &mut NodeContents<D, L> {
        match self.arena.get_mut(node.0) {
            None => panic!("Forest - node has been deleted (mut)!"),
            Some(node) => node,
        }
    }

    fn leaf(&self, node: Node<D, L>) -> &L {
        match &self.get(node).children {
            NodeChildren::Leaf(leaf) => leaf,
            NodeChildren::Branch(_) => panic!("Forest - branch nodes do not have leaf data"),
        }
    }

    fn leaf_mut(&mut self, node: Node<D, L>) -> &mut L {
        match &mut self.get_mut(node).children {
            NodeChildren::Leaf(leaf) => leaf,
            NodeChildren::Branch(_) => panic!("Forest - branch nodes do not have leaf data (mut)"),
        }
    }

    fn children(&self, node: Node<D, L>) -> &[Node<D, L>] {
        match &self.get(node).children {
            NodeChildren::Leaf(_) => panic!("Forest - leaf nodes do not have children"),
            NodeChildren::Branch(children) => children,
        }
    }

    fn children_mut(&mut self, node: Node<D, L>) -> &mut Vec<Node<D, L>> {
        match &mut self.get_mut(node).children {
            NodeChildren::Leaf(_) => panic!("Forest - leaf nodes do not have children (mut)"),
            NodeChildren::Branch(children) => children,
        }
    }

    fn siblings(&self, node: Node<D, L>) -> &[Node<D, L>] {
        let parent = self.get(node).parent;
        match parent {
            None => &self.roots,
            Some(parent) => self.children(parent),
        }
    }

    fn siblings_mut(&mut self, node: Node<D, L>) -> &mut Vec<Node<D, L>> {
        let parent = self.get(node).parent;
        match parent {
            None => &mut self.roots,
            Some(parent) => self.children_mut(parent),
        }
    }

    fn sibling_index(&self, node: Node<D, L>) -> usize {
        self.siblings(node)
            .iter()
            .position(|t| *t == node)
            .expect("Forest - missing child")
    }
}

impl<D, L> Node<D, L> {
    fn new(index: Index) -> Node<D, L> {
        Node(index, PhantomData)
    }

    /// Check if this node is still valid (has not been deleted).
    pub fn is_valid(self, f: &Forest<D, L>) -> bool {
        f.arena.contains(self.0)
    }

    /// Get this node's parent. Returns `None` if already at the root.
    pub fn parent(self, f: &Forest<D, L>) -> Option<Node<D, L>> {
        f.get(self).parent
    }

    /// Get the root of the tree containing this node. (This is the same as
    /// calling `parent()` repeatedly.)
    pub fn root(self, f: &Forest<D, L>) -> Node<D, L> {
        let mut root = self;
        while let Some(parent) = root.parent(f) {
            root = parent;
        }
        root
    }

    /// Get the data for this node.
    pub fn data(self, f: &Forest<D, L>) -> &D {
        &f.get(self).data
    }

    /// Mutably get the data for this node.
    pub fn data_mut(self, f: &mut Forest<D, L>) -> &mut D {
        &mut f.get_mut(self).data
    }

    /// Return `true` if this is a leaf node (containing `L`), or `false` if it's
    /// a branch node (containing children).
    pub fn is_leaf(self, f: &Forest<D, L>) -> bool {
        matches!(f.get(self).children, NodeChildren::Leaf(_))
    }

    /// Get the leaf data at this leaf node.
    ///
    /// # Panics
    ///
    /// Panics if this node is not a leaf.
    pub fn leaf(self, f: &Forest<D, L>) -> &L {
        f.leaf(self)
    }

    /// Mutably get the data at this leaf node.
    ///
    /// # Panics
    ///
    /// Panics if this node is not a leaf.
    pub fn leaf_mut(self, f: &mut Forest<D, L>) -> &mut L {
        f.leaf_mut(self)
    }

    /// Get this node's children.
    ///
    /// # Panics
    ///
    /// Panics if this is not a branch node.
    pub fn children(self, f: &Forest<D, L>) -> &[Node<D, L>] {
        f.children(self)
    }

    /// This node's siblings, in order, including itself.
    /// If this is a root node, its siblings are the roots.
    pub fn siblings(self, f: &Forest<D, L>) -> &[Node<D, L>] {
        f.siblings(self)
    }

    /// Determine this node’s index among its siblings.
    pub fn sibling_index(self, f: &Forest<D, L>) -> usize {
        f.sibling_index(self)
    }

    /// Detach this node from its parent. Afterwards, it will be a root node,
    /// and its parent will have one fewer child. If this node was already a
    /// root node, this is a no-op.
    pub fn detach(self, f: &mut Forest<D, L>) {
        if let Some(parent) = f.get(self).parent {
            let i = f.sibling_index(self);
            f.children_mut(parent).remove(i);
            f.get_mut(self).parent = None;
            f.roots.push(self);
        }
    }

    /// Insert `node` as this node's i'th child, after removing it from any
    /// previous parent it may have had.
    ///
    /// # Panics
    ///
    /// Panics if `child_index` is out of bounds, or if this node isn't a branch,
    /// or if you're trying to make a cycle like a douchebag.
    pub fn insert_child(self, f: &mut Forest<D, L>, child_index: usize, node: Node<D, L>) {
        if self.root(f) == node.root(f) {
            panic!("Forest - attempt to create cycle using `insert_child` thwarted");
        }
        let i = f.sibling_index(node);
        f.siblings_mut(node).remove(i);
        f.get_mut(node).parent = Some(self);
        f.children_mut(self).insert(child_index, node);
    }

    /// Swap the locations of nodes `self` and `other`.
    pub fn swap(self, f: &mut Forest<D, L>, other: Node<D, L>) {
        let i = f.sibling_index(self);
        let j = f.sibling_index(other);
        let self_parent = f.get(self).parent;
        let other_parent = f.get(other).parent;
        f.siblings_mut(self)[i] = other;
        f.siblings_mut(other)[j] = self;
        f.get_mut(self).parent = other_parent;
        f.get_mut(other).parent = self_parent;
    }

    /// Remove this node from its parent (if any), and delete it
    /// and all of its descendants.
    pub fn delete(self, f: &mut Forest<D, L>) {
        self.detach(f);
        f.roots.retain(|r| *r != self);
        let mut to_delete = vec![self];
        while let Some(node) = to_delete.pop() {
            let contents = &mut f.get_mut(node);
            if let NodeChildren::Branch(children) = &mut contents.children {
                to_delete.append(children);
            }
            f.arena.remove(node.0);
        }
    }
}

#[cfg(test)]
mod forest_tests {
    use super::*;
    use std::fmt::{Debug, Display};

    /// Verify and print a forest. Panic if verification fails. Verification checks:
    /// - Every node is accounted for in a tree in `roots`
    /// - node.parent is None if it's a root, or Some(parent) if it's a child
    struct Verifier<'a, D: Debug + Display, L: Debug + Display> {
        node_count: usize,
        display: String,
        forest: &'a Forest<D, L>,
    }

    impl<'a, D: Debug + Display, L: Debug + Display> Verifier<'a, D, L> {
        fn new(forest: &'a Forest<D, L>) -> Verifier<'a, D, L> {
            Verifier {
                node_count: 0,
                display: String::new(),
                forest,
            }
        }

        fn verify(mut self) -> String {
            // Walk each tree
            for (i, root) in self.forest.roots().iter().copied().enumerate() {
                self.verify_tree(root, None, root);
                assert_eq!(root.sibling_index(&self.forest), i);
                assert_eq!(root.siblings(&self.forest), self.forest.roots());
            }
            // Check that every node has been accounted for
            assert_eq!(self.node_count, self.forest.iter_nodes().len());
            self.display
        }

        fn verify_tree(
            &mut self,
            node: Node<D, L>,
            expected_parent: Option<Node<D, L>>,
            expected_root: Node<D, L>,
        ) {
            assert_eq!(node.parent(&self.forest), expected_parent);
            assert_eq!(node.root(&self.forest), expected_root);
            assert!(node.is_valid(&self.forest));
            self.display.push('(');
            self.display
                .push_str(&format!("{}", node.data(&self.forest)));
            self.node_count += 1;
            if node.is_leaf(&self.forest) {
                self.display
                    .push_str(&format!(" {}", node.leaf(&self.forest)));
            } else {
                for (i, child) in node.children(&self.forest).iter().copied().enumerate() {
                    self.display.push(' ');
                    self.verify_tree(child, Some(node), expected_root);
                    assert_eq!(child.sibling_index(&self.forest), i);
                    assert_eq!(child.siblings(&self.forest), node.children(&self.forest));
                }
            }
            self.display.push(')');
        }
    }

    fn verify_and_print<D: Debug + Display, L: Debug + Display>(forest: &Forest<D, L>) -> String {
        Verifier::new(forest).verify()
    }

    fn make_mirror(forest: &mut Forest<u32, char>, height: u32, id: u32) -> Node<u32, char> {
        if height == 0 {
            forest.new_leaf(id, 'a')
        } else {
            let parent = forest.new_branch(id);
            for i in 0..height {
                let child = make_mirror(forest, i, id + 2_u32.pow(i));
                parent.insert_child(forest, i as usize, child);
            }
            parent
        }
    }

    #[test]
    fn test_leaf() {
        let mut forest = Forest::new();
        forest.new_leaf("data", "leaf");
        assert_eq!(verify_and_print(&forest), "(data leaf)");
    }

    #[test]
    fn test_branch() {
        let mut f = Forest::new();
        let parent = f.new_branch("parent");
        let elder_sister = f.new_leaf("Sister", "elder");
        let younger_sister = f.new_leaf("sister", "younger");
        parent.insert_child(&mut f, 0, elder_sister);
        parent.insert_child(&mut f, 1, younger_sister);
        assert_eq!(
            verify_and_print(&f),
            "(parent (Sister elder) (sister younger))"
        );
    }

    #[test]
    fn test_mirror() {
        let mut f = Forest::new();
        make_mirror(&mut f, 3, 0);
        assert_eq!(
            verify_and_print(&f),
            "(0 (1 a) (2 (3 a)) (4 (5 a) (6 (7 a))))"
        );
    }

    #[test]
    fn test_mutation() {
        let mut f = Forest::new();
        let root = make_mirror(&mut f, 3, 0);
        *root.data_mut(&mut f) = 100;
        *root.children(&f)[1].children(&f)[0].leaf_mut(&mut f) = 'b';
        let last_child = root.children(&f)[2];
        *last_child.children(&f)[0].leaf_mut(&mut f) = 'c';
        *last_child.children(&f)[1].children(&f)[0].leaf_mut(&mut f) = 'd';
        assert_eq!(
            verify_and_print(&f),
            "(100 (1 a) (2 (3 b)) (4 (5 c) (6 (7 d))))"
        );
    }

    #[test]
    fn test_modification() {
        let mut f = Forest::<&'static str, u32>::new();
        let kid = f.new_branch("kid");
        let mama = f.new_branch("mama");
        kid.insert_child(&mut f, 0, mama);
        let papa = f.new_branch("papa");
        kid.insert_child(&mut f, 1, papa);
        let gram = f.new_leaf("gram", 99);
        mama.insert_child(&mut f, 0, gram);
        let gramp = f.new_leaf("gramp", 100);
        mama.insert_child(&mut f, 0, gramp);
        let ogram = f.new_leaf("ogram", 79);
        papa.insert_child(&mut f, 0, ogram);
        let ogramp = f.new_leaf("ogramp", 80);
        papa.insert_child(&mut f, 0, ogramp);
        assert_eq!(
            verify_and_print(&f),
            "(kid (mama (gramp 100) (gram 99)) (papa (ogramp 80) (ogram 79)))"
        );

        mama.detach(&mut f);
        mama.detach(&mut f);
        assert_eq!(
            verify_and_print(&f),
            "(kid (papa (ogramp 80) (ogram 79)))(mama (gramp 100) (gram 99))"
        );

        kid.insert_child(&mut f, 0, gramp);
        assert_eq!(
            verify_and_print(&f),
            "(kid (gramp 100) (papa (ogramp 80) (ogram 79)))(mama (gram 99))"
        );

        kid.swap(&mut f, mama);
        gramp.swap(&mut f, gram);
        assert_eq!(
            verify_and_print(&f),
            "(mama (gramp 100))(kid (gram 99) (papa (ogramp 80) (ogram 79)))"
        );

        papa.delete(&mut f);
        assert!(!papa.is_valid(&f));
        assert!(!ogramp.is_valid(&f));
        assert!(!ogram.is_valid(&f));
        assert_eq!(verify_and_print(&f), "(mama (gramp 100))(kid (gram 99))");
    }

    // Error Testing //

    #[test]
    #[should_panic(expected = "Forest - leaf nodes do not have children")]
    fn test_children_panic() {
        let mut f = Forest::<(), ()>::new();
        let tree = f.new_leaf((), ());
        tree.children(&f);
    }

    #[test]
    #[should_panic(expected = "Forest - branch nodes do not have leaf data")]
    fn test_leaf_panic() {
        let mut f = Forest::<(), ()>::new();
        let tree = f.new_branch(());
        tree.leaf(&f);
    }

    #[test]
    #[should_panic(expected = "Forest - branch nodes do not have leaf data (mut)")]
    fn test_leaf_mut_panic() {
        let mut f = Forest::<(), ()>::new();
        let tree = f.new_branch(());
        tree.leaf_mut(&mut f);
    }

    #[test]
    #[should_panic(expected = "insertion index")]
    fn test_insert_oob_panic() {
        let mut f = Forest::<(), ()>::new();
        let tree = f.new_branch(());
        let child = f.new_leaf((), ());
        tree.insert_child(&mut f, 1, child);
    }

    #[test]
    #[should_panic(expected = "Forest - leaf nodes do not have children")]
    fn test_insert_leaf_panic() {
        let mut f = Forest::<(), ()>::new();
        let leaf = f.new_leaf((), ());
        let child = f.new_leaf((), ());
        leaf.insert_child(&mut f, 0, child);
    }

    #[test]
    #[should_panic(expected = "thwarted")]
    fn test_cycle() {
        let mut f = Forest::<u32, u32>::new();
        let tree = f.new_branch(0);
        tree.insert_child(&mut f, 0, tree);
    }

    #[test]
    #[should_panic(expected = "thwarted")]
    fn test_deeper_cycle() {
        let mut f = Forest::<u32, u32>::new();
        let n1 = f.new_branch(1);
        let n2 = f.new_branch(2);
        let n3 = f.new_branch(3);
        let n4 = f.new_branch(4);
        n1.insert_child(&mut f, 0, n2);
        n2.insert_child(&mut f, 0, n3);
        n3.insert_child(&mut f, 0, n4);
        n3.insert_child(&mut f, 0, n2);
    }

    #[test]
    #[should_panic(expected = "Forest - node has been deleted!")]
    fn test_use_after_free_panic() {
        let mut f = Forest::<u32, u32>::new();
        let tree = f.new_branch(0);
        tree.delete(&mut f);
        tree.data(&f);
    }
}

/*
 TODO: This belongs in ast now

/// Save a bookmark to return to later.
pub fn bookmark(self) -> Bookmark<D, L> {
    Bookmark(self.0, self.1)
}

/// Jump to a previously saved bookmark, as long as that bookmark’s
/// node is present somewhere in this tree. This will work even if
/// the Tree has been modified since the bookmark was created.
/// However, it will return false if the bookmark’s node has since
/// been deleted, or if it is currently located in a different tree.
pub fn lookup_bookmark(self, f: &Forest<D, L>, bookmark: Bookmark<D, L>) -> Option<Node<D, L>> {
    if f.arena.contains(bookmark.0) {
        let bookmark_node = Node(bookmark.0, bookmark.1);
        if bookmark_node.root(f).0 == self.root(f).0 {
            // The bookmark exists, and is in this tree.
            Some(bookmark_node)
        } else {
            // The bookmark exists, but is in a different tree.
            None
        }
    } else {
        // The bookmark has been deleted.
        None
    }
}
*/
