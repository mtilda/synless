use slab::Slab;
use std::rc::Rc;
use uuid::Uuid;
//#[cfg(test)]
use super::node::{Key, Node};
use super::node_slab::NodeSlab;
use super::tree2::Tree;
use std::cell::{Ref, RefCell, RefMut};
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

// INVARIANTS:
// - children and parents agree

/// All [Trees](Tree) belong to a Forest.
///
/// It is your responsibility to ensure that Trees are kept with the Forest they came from. The
/// methods on Trees may panic if you use them on a different Forest.
pub struct Forest<D, L>(pub(super) Rc<RefCell<NodeSlab<D, L>>>);

impl<D, L> Clone for Forest<D, L> {
    fn clone(&self) -> Forest<D, L> {
        Forest(self.0.clone())
    }
}

impl<D, L> Forest<D, L> {
    /// Construct a new forest.
    pub fn new() -> Forest<D, L> {
        Forest(Rc::new(RefCell::new(NodeSlab::new())))
    }

    /// Construct a new leaf.
    pub fn new_leaf(&self, leaf: L) -> Tree<D, L> {
        let node = Node::new_leaf(leaf);

        #[cfg(test)]
        self.0.inc_refcount();

        let mut slab = self.0.borrow_mut();
        let leaf_key = slab.insert(node);

        Tree::new(&self.0, leaf_key)
    }

    /// Construct a new branch.
    pub fn new_branch(&self, data: D, children: Vec<Tree<D, L>>) -> Tree<D, L> {
        let child_keys = children.iter().map(|tree| tree.key).collect::<Vec<_>>();
        let node = Node::new_branch(data, child_keys);

        #[cfg(test)]
        self.0.inc_refcount();

        // Need to make these changes:
        //   parent = new node(children);
        //   for each child: child.parent = parent;
        let mut slab = self.0.borrow_mut();
        let parent_key = slab.insert(node);
        for tree in children.into_iter() {
            let key = tree.key;
            // Don't let `Drop` get called, or it will delete the tree from under us!
            mem::forget(tree);
            slab[key].parent = Some(parent_key);
        }

        Tree::new(&self.0, parent_key)
    }
}
