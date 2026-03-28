use core::{marker::PhantomData};

use super::traits::{Get, GetMut};

#[allow(dead_code)]
pub struct RbTree<Tag, T: Copy> {
    root: Option<T>,
    min: Option<T>,
    _tag: PhantomData<Tag>,
}

#[allow(dead_code)]
pub trait Linkable<Tag, T> {
    fn links(&self) -> &Links<Tag, T>;
    fn links_mut(&mut self) -> &mut Links<Tag, T>;
}

pub trait Compare<Tag, T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering;
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Links<Tag, T> {
    parent: Option<T>,
    left: Option<T>,
    right: Option<T>,
    color: Color,
    _tag: PhantomData<Tag>,
}

#[allow(dead_code)]
impl<Tag, T> Links<Tag, T> {
    pub fn new() -> Self {
        Self {
            parent: None,
            left: None,
            right: None,
            color: Color::Red,
            _tag: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Color {
    Red,
    Black,
}

#[allow(dead_code)]
impl<Tag, T: Copy + PartialEq> RbTree<Tag, T>
{
    pub const fn new() -> Self {
        Self {
            root: None,
            min: None,
            _tag: PhantomData,
        }
    }

    pub fn insert<S: Get<T> + GetMut<T>>(&mut self, id: T, storage: &mut S) -> Result<(), ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T>,{
        let already_linked = {
            let node = storage.get(id).ok_or(())?;
            let links = node.links();
            self.root == Some(id)
                || links.parent.is_some()
                || links.left.is_some()
                || links.right.is_some()
        };

        if already_linked {
            self.remove(id, storage)?;
        }

        let mut last = None;

        {
            let node = storage.get(id).ok_or(())?;
            let mut current = self.root;

            while let Some(current_id) = current {
                last = current;
                let current_node = storage.get(current_id).ok_or(())?;
                let go_left = node.cmp(current_node) == core::cmp::Ordering::Less;

                current = if go_left {
                    current_node.links().left
                } else {
                    current_node.links().right
                };
            }
        }

        {
            let node = storage.get_mut(id).ok_or(())?.links_mut();
            node.parent = last;
            node.left = None;
            node.right = None;
            node.color = Color::Red;
        }

        match last {
            None => self.root = Some(id),
            Some(last_id) => {
                if let (Some(node), Some(last)) = storage.get2_mut(id, last_id) {
                    if node.cmp(last) == core::cmp::Ordering::Less {
                        last.links_mut().left = Some(id);
                    } else {
                        last.links_mut().right = Some(id);  
                    }
                }
            }
        }

        if let Some(min_id) = self.min {
            let node = storage.get(id).ok_or(())?;
            let min_node = storage.get(min_id).ok_or(())?;
            if node.cmp(min_node) == core::cmp::Ordering::Less {
                self.min = Some(id);
            }
        } else {
            self.min = Some(id);
        }

        self.insert_fixup(id, storage)
    }

    pub fn remove<S: Get<T> + GetMut<T>>(&mut self, id: T, storage: &mut S) -> Result<(), ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T> {
        let (node_left, node_right, node_parent, node_is_red) = {
            let node = storage.get(id).ok_or(())?;
            (
                node.links().left,
                node.links().right,
                node.links().parent,
                matches!(node.links().color, Color::Red),
            )
        };

        let mut succ_was_red = node_is_red;
        let child: Option<T>;
        let child_parent: Option<T>;

        if node_left.is_none() {
            child = node_right;
            child_parent = node_parent;

            self.transplant(id, node_right, storage)?;
        } else if node_right.is_none() {
            child = node_left;
            child_parent = node_parent;

            self.transplant(id, node_left, storage)?;
        } else {
            let right_id = node_right.ok_or(())?;
            let succ = self.minimum(right_id, storage)?;
            let succ_right = storage.get(succ).and_then(|n| n.links().right);
            let succ_parent = storage.get(succ).and_then(|n| n.links().parent);

            succ_was_red = storage
                .get(succ)
                .map_or(false, |n| matches!(n.links().color, Color::Red));
            child = succ_right;

            if succ_parent == Some(id) {
                child_parent = Some(succ);
            } else {
                self.transplant(succ, succ_right, storage)?;

                if let (Some(succ_node), Some(right_node)) = storage.get2_mut(succ, right_id) {
                    succ_node.links_mut().right = Some(right_id);
                    right_node.links_mut().parent = Some(succ);
                } else {
                    return Err(());
                }

                child_parent = succ_parent;
            }

            self.transplant(id, Some(succ), storage)?;

            let left_id = node_left.ok_or(())?;

            if let (Some(succ_node), Some(left_node)) = storage.get2_mut(succ, left_id) {
                succ_node.links_mut().left = Some(left_id);
                left_node.links_mut().parent = Some(succ);
            } else {
                return Err(());
            }

            if let Some(succ_node) = storage.get_mut(succ) {
                succ_node.links_mut().color = if node_is_red {
                    Color::Red
                } else {
                    Color::Black
                };
            } else {
                return Err(());
            }
        }

        if !succ_was_red {
            self.delete_fixup(child, child_parent, storage)?;
        }

        if self.min == Some(id) {
            self.min = match self.root {
                Some(root_id) => Some(self.minimum(root_id, storage)?),
                None => None,
            };
        }

        Ok(())
    }

    pub fn min(&self) -> Option<T> {
        self.min
    }

    fn insert_fixup<S: Get<T> + GetMut<T>>(&mut self, mut id: T, storage: &mut S) -> Result<(), ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T>, {
        while let Some(parent) = storage.get(id).and_then(|n| n.links().parent)
            && storage
                .get(parent)
                .map_or(false, |n| matches!(n.links().color, Color::Red))
        {
            let grandparent = storage
                .get(parent)
                .and_then(|n| n.links().parent)
                .ok_or(())?;

            // Is left child node
            if storage
                .get(grandparent)
                .map_or(false, |n| n.links().left == Some(parent))
            {
                // Uncle node must be the right child node
                let uncle = storage.get(grandparent).and_then(|n| n.links().right);

                if let Some(uncle_id) = uncle
                    && storage
                        .get(uncle_id)
                        .map_or(false, |n| matches!(n.links().color, Color::Red))
                {
                    // Parent and uncle nodes are red
                    if let (Some(parent_node), Some(uncle_node), Some(grandparent_node)) =
                        storage.get3_mut(parent, uncle_id, grandparent)
                    {
                        parent_node.links_mut().color = Color::Black;
                        uncle_node.links_mut().color = Color::Black;
                        grandparent_node.links_mut().color = Color::Red;
                    }
                    id = grandparent;
                } else {
                    // Uncle node is black
                    if storage
                        .get(parent)
                        .map_or(false, |n| n.links().right == Some(id))
                    {
                        let old_parent = parent;
                        self.rotate_left(parent, id, storage)?;
                        id = old_parent;
                    }

                    let parent = storage.get(id).and_then(|n| n.links().parent).ok_or(())?;
                    let grandparent = storage
                        .get(parent)
                        .and_then(|n| n.links().parent)
                        .ok_or(())?;

                    if let (Some(parent_node), Some(grandparent_node)) =
                        storage.get2_mut(parent, grandparent)
                    {
                        parent_node.links_mut().color = Color::Black;
                        grandparent_node.links_mut().color = Color::Red;
                    }
                    self.rotate_right(grandparent, parent, storage)?;
                    break;
                }
            } else {
                // Uncle node must be the left child
                let uncle = storage.get(grandparent).and_then(|n| n.links().left);

                if let Some(uncle_id) = uncle
                    && storage
                        .get(uncle_id)
                        .map_or(false, |n| matches!(n.links().color, Color::Red))
                {
                    // Parent and uncle nodes are red
                    if let (Some(parent_node), Some(uncle_node), Some(grandparent_node)) =
                        storage.get3_mut(parent, uncle_id, grandparent)
                    {
                        parent_node.links_mut().color = Color::Black;
                        uncle_node.links_mut().color = Color::Black;
                        grandparent_node.links_mut().color = Color::Red;
                    }
                    id = grandparent;
                } else {
                    // Uncle node is black
                    if storage
                        .get(parent)
                        .map_or(false, |n| n.links().left == Some(id))
                    {
                        let old_parent = parent;
                        self.rotate_right(parent, id, storage)?;
                        id = old_parent;
                    }

                    let parent = storage.get(id).and_then(|n| n.links().parent).ok_or(())?;
                    let grandparent = storage
                        .get(parent)
                        .and_then(|n| n.links().parent)
                        .ok_or(())?;

                    if let (Some(parent_node), Some(grandparent_node)) =
                        storage.get2_mut(parent, grandparent)
                    {
                        parent_node.links_mut().color = Color::Black;
                        grandparent_node.links_mut().color = Color::Red;
                    }
                    self.rotate_left(grandparent, parent, storage)?;
                    break;
                }
            }
        }

        if let Some(root_id) = self.root {
            if let Some(root_node) = storage.get_mut(root_id) {
                root_node.links_mut().color = Color::Black;
            }
        }

        Ok(())
    }

    fn delete_fixup<S: Get<T> + GetMut<T>>(
        &mut self,
        mut id: Option<T>,
        mut parent: Option<T>,
        storage: &mut S,
    ) -> Result<(), ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T>, {
        let is_red = |node_id: Option<T>, storage: &S| -> bool {
            node_id
                .and_then(|id| storage.get(id))
                .map_or(false, |n| matches!(n.links().color, Color::Red))
        };

        let is_black = |node_id: Option<T>, storage: &S| -> bool { !is_red(node_id, storage) };
        
        while id != self.root && is_black(id, storage) {
            let parent_id = parent.ok_or(())?;

            let is_left_child = storage
                .get(parent_id)
                .map_or(false, |n| n.links().left == id);

            if is_left_child {
                let mut sibling_opt = storage.get(parent_id).and_then(|n| n.links().right);

                if is_red(sibling_opt, storage) {
                    let sibling_id = sibling_opt.ok_or(())?;
                    // Color sibling node black and parent node red, rotate
                    if let (Some(sib), Some(par)) = storage.get2_mut(sibling_id, parent_id) {
                        sib.links_mut().color = Color::Black;
                        par.links_mut().color = Color::Red;
                    } else {
                        return Err(());
                    }
                    self.rotate_left(parent_id, sibling_id, storage)?;
                    sibling_opt = storage.get(parent_id).and_then(|n| n.links().right);
                }

                // Sibling node is black
                let sibling_id = sibling_opt.ok_or(())?;
                let sib_left = storage.get(sibling_id).and_then(|n| n.links().left);
                let sib_right = storage.get(sibling_id).and_then(|n| n.links().right);

                if is_black(sib_left, storage) && is_black(sib_right, storage) {
                    // Color sibling node red and move up
                    if let Some(sib) = storage.get_mut(sibling_id) {
                        sib.links_mut().color = Color::Red;
                    } else {
                        return Err(());
                    }
                    id = Some(parent_id);
                    parent = storage.get(parent_id).and_then(|n| n.links().parent);
                } else {
                    // Sibling's left node is red
                    if is_black(sib_right, storage) {
                        let sib_left_id = sib_left.ok_or(())?;
                        if let (Some(sib), Some(left)) = storage.get2_mut(sibling_id, sib_left_id) {
                            sib.links_mut().color = Color::Red;
                            left.links_mut().color = Color::Black;
                        } else {
                            return Err(());
                        }
                        self.rotate_right(sibling_id, sib_left_id, storage)?;
                        sibling_opt = storage.get(parent_id).and_then(|n| n.links().right);
                    }

                    // Sibling's right child node is red
                    let sibling_id = sibling_opt.ok_or(())?;
                    let parent_is_red = storage
                        .get(parent_id)
                        .map_or(false, |n| matches!(n.links().color, Color::Red));

                    if let Some(sib) = storage.get_mut(sibling_id) {
                        sib.links_mut().color = if parent_is_red {
                            Color::Red
                        } else {
                            Color::Black
                        };
                    }
                    if let Some(par) = storage.get_mut(parent_id) {
                        par.links_mut().color = Color::Black;
                    }

                    let sib_right = storage.get(sibling_id).and_then(|n| n.links().right);
                    if let Some(sib_right_id) = sib_right {
                        if let Some(right) = storage.get_mut(sib_right_id) {
                            right.links_mut().color = Color::Black;
                        }
                    }

                    self.rotate_left(parent_id, sibling_id, storage)?;
                    id = self.root;
                    break;
                }
            } else {
                let mut sibling_opt = storage.get(parent_id).and_then(|n| n.links().left);

                if is_red(sibling_opt, storage) {
                    let sibling_id = sibling_opt.ok_or(())?;
                    if let (Some(sib), Some(par)) = storage.get2_mut(sibling_id, parent_id) {
                        sib.links_mut().color = Color::Black;
                        par.links_mut().color = Color::Red;
                    } else {
                        return Err(());
                    }
                    self.rotate_right(parent_id, sibling_id, storage)?;
                    sibling_opt = storage.get(parent_id).and_then(|n| n.links().left);
                }

                // Sibling node is black
                let sibling_id = sibling_opt.ok_or(())?;
                let sib_left = storage.get(sibling_id).and_then(|n| n.links().left);
                let sib_right = storage.get(sibling_id).and_then(|n| n.links().right);

                if is_black(sib_left, storage) && is_black(sib_right, storage) {
                    if let Some(sib) = storage.get_mut(sibling_id) {
                        sib.links_mut().color = Color::Red;
                    } else {
                        return Err(());
                    }
                    id = Some(parent_id);
                    parent = storage.get(parent_id).and_then(|n| n.links().parent);
                } else {
                    // Sibling's right node is red
                    if is_black(sib_left, storage) {
                        let sib_right_id = sib_right.ok_or(())?;
                        if let (Some(sib), Some(right)) = storage.get2_mut(sibling_id, sib_right_id)
                        {
                            sib.links_mut().color = Color::Red;
                            right.links_mut().color = Color::Black;
                        } else {
                            return Err(());
                        }
                        self.rotate_left(sibling_id, sib_right_id, storage)?;
                        sibling_opt = storage.get(parent_id).and_then(|n| n.links().left);
                    }

                    // Sibling's left child node is red
                    let sibling_id = sibling_opt.ok_or(())?;
                    let parent_is_red = storage
                        .get(parent_id)
                        .map_or(false, |n| matches!(n.links().color, Color::Red));

                    if let Some(sib) = storage.get_mut(sibling_id) {
                        sib.links_mut().color = if parent_is_red {
                            Color::Red
                        } else {
                            Color::Black
                        };
                    }
                    if let Some(par) = storage.get_mut(parent_id) {
                        par.links_mut().color = Color::Black;
                    }

                    let sib_left = storage.get(sibling_id).and_then(|n| n.links().left);
                    if let Some(sib_left_id) = sib_left {
                        if let Some(left) = storage.get_mut(sib_left_id) {
                            left.links_mut().color = Color::Black;
                        }
                    }

                    self.rotate_right(parent_id, sibling_id, storage)?;
                    id = self.root;
                    break;
                }
            }
        }

        // Color the root node black
        if let Some(id) = id {
            if let Some(node) = storage.get_mut(id) {
                node.links_mut().color = Color::Black;
            }
        }

        Ok(())
    }

    fn minimum<S: Get<T>>(&self, mut id: T, storage: &S) -> Result<T, ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T>, {
        loop {
            let left = storage.get(id).ok_or(())?.links().left;
            match left {
                Some(left_id) => id = left_id,
                None => return Ok(id),
            }
        }
    }

    fn transplant<S: Get<T> + GetMut<T>>(&mut self, u: T, v: Option<T>, storage: &mut S) -> Result<(), ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T>, {
        let u_parent = storage.get(u).and_then(|n| n.links().parent);

        match u_parent {
            None => self.root = v,
            Some(parent_id) => {
                if let Some(parent_node) = storage.get_mut(parent_id) {
                    if parent_node.links().left == Some(u) {
                        parent_node.links_mut().left = v;
                    } else {
                        parent_node.links_mut().right = v;
                    }
                } else {
                    return Err(());
                }
            }
        }

        if let Some(v_id) = v {
            if let Some(v_node) = storage.get_mut(v_id) {
                v_node.links_mut().parent = u_parent;
            } else {
                return Err(());
            }
        }

        Ok(())
    }

    fn rotate_right<S: Get<T> + GetMut<T>>(&mut self, pivot: T, left: T, storage: &mut S) -> Result<(), ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T>, {
        if pivot == left {
            return Err(());
        }

        let (right, parent) =
            if let (Some(pivot_node), Some(left_node)) = storage.get2_mut(pivot, left) {
                // Add left child's right subtree as pivot's left subtree
                pivot_node.links_mut().left = left_node.links().right;

                // Add pivot's parent as left child's parent
                left_node.links_mut().parent = pivot_node.links().parent;

                let old_right = left_node.links().right;

                // Set pivot as the right child of left child
                left_node.links_mut().right = Some(pivot);

                let old_parent = pivot_node.links().parent;

                // Set pivot's parent to left child
                pivot_node.links_mut().parent = Some(left);

                (old_right, old_parent)
            } else {
                return Err(());
            };

        if let Some(right_id) = right {
            if let Some(right_node) = storage.get_mut(right_id) {
                right_node.links_mut().parent = Some(pivot);
            }
        }

        match parent {
            None => self.root = Some(left),
            Some(parent_id) => {
                if let Some(parent_node) = storage.get_mut(parent_id) {
                    if parent_node.links().left == Some(pivot) {
                        parent_node.links_mut().left = Some(left);
                    } else {
                        parent_node.links_mut().right = Some(left);
                    }
                } else {
                    return Err(());
                }
            }
        }

        Ok(())
    }

    fn rotate_left<S: Get<T> + GetMut<T>>(&mut self, pivot: T, right: T, storage: &mut S) -> Result<(), ()>
    where <S as Get<T>>::Output: Linkable<Tag, T> + Compare<Tag, T>, {
        if pivot == right {
            return Err(());
        }

        let (left, parent) =
            if let (Some(pivot_node), Some(right_node)) = storage.get2_mut(pivot, right) {
                // Add right child's left subtree as pivot's right subtree
                pivot_node.links_mut().right = right_node.links().left;

                // Add pivot's parent as right child's parent
                right_node.links_mut().parent = pivot_node.links().parent;

                let old_left = right_node.links().left;

                // Set pivot as the left child of right child
                right_node.links_mut().left = Some(pivot);

                let old_parent = pivot_node.links().parent;

                // Set pivot's parent to right child
                pivot_node.links_mut().parent = Some(right);

                (old_left, old_parent)
            } else {
                return Err(());
            };

        if let Some(left_id) = left {
            if let Some(left_node) = storage.get_mut(left_id) {
                left_node.links_mut().parent = Some(pivot);
            }
        }

        match parent {
            None => self.root = Some(right),
            Some(parent_id) => {
                if let Some(parent_node) = storage.get_mut(parent_id) {
                    if parent_node.links().left == Some(pivot) {
                        parent_node.links_mut().left = Some(right);
                    } else {
                        parent_node.links_mut().right = Some(right);
                    }
                } else {
                    return Err(());
                }
            }
        }
        Ok(())
    }
}

// TESTING ------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::{Get, GetMut};
    use std::borrow::Borrow;
    use std::collections::HashSet;

    struct Tree;

    struct Node {
        key: i32,
        links: Links<Tree, usize>,
    }

    impl Node {
        fn new(key: i32) -> Self {
            Self {
                key,
                links: Links::new(),
            }
        }
    }

    impl Compare<Tree, usize> for Node {
        fn cmp(&self, other: &Self) -> core::cmp::Ordering {
            self.key.cmp(&other.key)
        }
    }

    impl Linkable<Tree, usize> for Node {
        fn links(&self) -> &Links<Tree, usize> {
            &self.links
        }

        fn links_mut(&mut self) -> &mut Links<Tree, usize> {
            &mut self.links
        }
    }

    struct NodeStore {
        nodes: Vec<Node>,
    }

    impl NodeStore {
        fn new(keys: &[i32]) -> Self {
            Self {
                nodes: keys.iter().copied().map(Node::new).collect(),
            }
        }
    }

    impl Get<usize> for NodeStore {
        type Output = Node;

        fn get<K: Borrow<usize>>(&self, index: K) -> Option<&Self::Output> {
            self.nodes.get(*index.borrow())
        }
    }

    impl GetMut<usize> for NodeStore {
        fn get_mut<K: Borrow<usize>>(&mut self, index: K) -> Option<&mut Self::Output> {
            self.nodes.get_mut(*index.borrow())
        }

        fn get2_mut<K: Borrow<usize>>(
            &mut self,
            index1: K,
            index2: K,
        ) -> (Option<&mut Self::Output>, Option<&mut Self::Output>) {
            if *index1.borrow() == *index2.borrow() {
                return (None, None);
            }

            let ptr = self.nodes.as_ptr();

            return unsafe {
                (
                    Some(&mut *(ptr.add(*index1.borrow()) as *mut Self::Output)),
                    Some(&mut *(ptr.add(*index2.borrow()) as *mut Self::Output)),
                )
            };
        }

        fn get3_mut<K: Borrow<usize>>(
            &mut self,
            index1: K,
            index2: K,
            index3: K,
        ) -> (
            Option<&mut Self::Output>,
            Option<&mut Self::Output>,
            Option<&mut Self::Output>,
        ) {
            if *index1.borrow() == *index2.borrow()
                || *index1.borrow() == *index3.borrow()
                || *index2.borrow() == *index3.borrow()
            {
                return (None, None, None);
            }

            let ptr = self.nodes.as_ptr();
            return unsafe {
                (
                    Some(&mut *(ptr.add(*index1.borrow()) as *mut Self::Output)),
                    Some(&mut *(ptr.add(*index2.borrow()) as *mut Self::Output)),
                    Some(&mut *(ptr.add(*index3.borrow()) as *mut Self::Output)),
                )
            };
        }
    }

    fn validate_tree(tree: &RbTree<Tree, usize>, store: &NodeStore, expected: &[i32]) {
        let mut visited = HashSet::new();

        if let Some(root_id) = tree.root {
            let root = store.get(root_id).expect("root missing from store");
            assert!(matches!(root.links().color, Color::Black));
            assert_eq!(root.links().parent, None);
        }

        let (count, _) = validate_node(tree.root, store, &mut visited, expected);
        assert_eq!(count, expected.len());

        if !expected.is_empty() {
            let min = tree_min_key(tree, store).expect("non-empty tree must contain a min.");
            assert_eq!(min, expected[0]);
        }
    }

    fn tree_min_key(tree: &RbTree<Tree, usize>, store: &NodeStore) -> Option<i32> {
        tree.min().map(|id| store.get(id).expect("min missing").key)
    }

    fn validate_node(
        id: Option<usize>,
        store: &NodeStore,
        visited: &mut HashSet<usize>,
        expected: &[i32],
    ) -> (usize, usize) {
        let Some(id) = id else {
            return (0, 1);
        };

        assert!(visited.insert(id));

        let node = store.get(id).expect("node missing from store");

        let left = node.links().left;
        let right = node.links().right;

        if matches!(node.links().color, Color::Red) {
            if let Some(left_id) = left {
                let left_node = store.get(left_id).expect("left missing");
                assert!(matches!(left_node.links().color, Color::Black));
            }
            if let Some(right_id) = right {
                let right_node = store.get(right_id).expect("right missing");
                assert!(matches!(right_node.links().color, Color::Black));
            }
        }

        if let Some(left_id) = left {
            let left_node = store.get(left_id).expect("left missing");
            assert_eq!(left_node.links().parent, Some(id));
        }
        if let Some(right_id) = right {
            let right_node = store.get(right_id).expect("right missing");
            assert_eq!(right_node.links().parent, Some(id));
        }

        let (left_count, left_bh) = validate_node(left, store, visited, &expected);
        assert_eq!(
            node.key, expected[left_count],
            "expected key {}, found {}",
            expected[left_count], node.key
        );
        let (right_count, right_bh) =
            validate_node(right, store, visited, &expected[1 + left_count..]);

        assert_eq!(
            left_bh, right_bh,
            "black height mismatch at node with key {}",
            node.key
        );

        let self_bh = if matches!(node.links().color, Color::Black) {
            left_bh + 1
        } else {
            left_bh
        };

        (1 + left_count + right_count, self_bh)
    }

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        *seed
    }

    fn shuffle(ids: &mut [usize]) {
        let mut seed = 0x6b8b_4567_9a1c_def0u64;
        for i in (1..ids.len()).rev() {
            let j = (lcg(&mut seed) % (i as u64 + 1)) as usize;
            ids.swap(i, j);
        }
    }

    #[test]
    fn insert_validates() {
        let keys: Vec<i32> = (0..200).collect();
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();
        let mut order: Vec<usize> = (0..keys.len()).collect();

        shuffle(&mut order);
        for id in order {
            tree.insert(id, &mut store).unwrap();
        }

        validate_tree(&tree, &store, &keys);
    }

    #[test]
    fn reinsert_same_id_is_stable() {
        let keys = vec![10, 5, 15];
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();

        tree.insert(0, &mut store).unwrap();
        tree.insert(1, &mut store).unwrap();
        tree.insert(2, &mut store).unwrap();

        // Reinsert existing node id. This should not create duplicate structural links.
        tree.insert(1, &mut store).unwrap();

        let mut expected = keys.clone();
        expected.sort();
        validate_tree(&tree, &store, &expected);
    }

    #[test]
    fn min_updates_on_insert_and_remove() {
        let keys = vec![10, 5, 15, 3, 7, 12, 18, 1, 6];
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();

        for id in 0..keys.len() {
            tree.insert(id, &mut store).unwrap();
        }

        let mut sorted_keys = keys.clone();
        sorted_keys.sort();

        validate_tree(&tree, &store, &sorted_keys);
        assert_eq!(tree_min_key(&tree, &store), Some(1));

        // Remove index 7 (key=1)
        tree.remove(7, &mut store).unwrap();
        sorted_keys.retain(|&x| x != 1);
        validate_tree(&tree, &store, &sorted_keys);
        assert_eq!(tree_min_key(&tree, &store), Some(3));

        // Remove index 8 (key=6)
        tree.remove(8, &mut store).unwrap();
        sorted_keys.retain(|&x| x != 6);
        validate_tree(&tree, &store, &sorted_keys);
        assert_eq!(tree_min_key(&tree, &store), Some(3));

        // Remove index 3 (key=3)
        tree.remove(3, &mut store).unwrap();
        sorted_keys.retain(|&x| x != 3);
        validate_tree(&tree, &store, &sorted_keys);
        assert_eq!(tree_min_key(&tree, &store), Some(5));
    }

    #[test]
    fn remove_leaf_one_child_two_children() {
        let keys = vec![10, 5, 15, 3, 7, 12, 18, 1, 6];
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();

        for id in 0..keys.len() {
            tree.insert(id, &mut store).unwrap();
        }

        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        validate_tree(&tree, &store, &sorted_keys);

        // Remove node at index 4 (key=7)
        tree.remove(4, &mut store).unwrap();
        sorted_keys.retain(|&x| x != 7);
        validate_tree(&tree, &store, &sorted_keys);

        // Remove node at index 3 (key=3)
        tree.remove(3, &mut store).unwrap();
        sorted_keys.retain(|&x| x != 3);
        validate_tree(&tree, &store, &sorted_keys);

        // Remove node at index 7 (key=1)
        tree.remove(7, &mut store).unwrap();
        sorted_keys.retain(|&x| x != 1);
        validate_tree(&tree, &store, &sorted_keys);
    }

    #[test]
    fn remove_root_with_two_children() {
        let keys = [8, 4, 12, 2, 6, 10, 14, 1, 3, 5, 7, 9, 11, 13, 15];
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();

        for id in 0..keys.len() {
            tree.insert(id, &mut store).unwrap();
        }

        let mut sorted_keys: Vec<i32> = keys.to_vec();
        sorted_keys.sort();
        validate_tree(&tree, &store, &sorted_keys);

        let root_id = tree.root.expect("root missing");
        let root_key = store.get(root_id).expect("root missing").key;

        tree.remove(root_id, &mut store).unwrap();
        sorted_keys.retain(|&x| x != root_key);
        validate_tree(&tree, &store, &sorted_keys);
    }

    #[test]
    fn remove_all_nodes() {
        let keys: Vec<i32> = (0..128).collect();
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        shuffle(&mut order);

        for id in &order {
            tree.insert(*id, &mut store).unwrap();
        }

        let mut remaining_keys = keys.clone();
        validate_tree(&tree, &store, &remaining_keys);

        for id in order {
            let removed_key = keys[id];
            tree.remove(id, &mut store).unwrap();
            remaining_keys.retain(|&k| k != removed_key);
            validate_tree(&tree, &store, &remaining_keys);
        }

        assert_eq!(tree.root, None);
    }

    #[test]
    fn interleaved_operations() {
        let keys: Vec<i32> = (0..100).collect();
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        shuffle(&mut order);

        // Build initial tree with 50 nodes
        let mut active_keys: Vec<i32> = Vec::new();
        for id in order.iter().take(50) {
            tree.insert(*id, &mut store).unwrap();
            active_keys.push(keys[*id]);
        }
        active_keys.sort();
        validate_tree(&tree, &store, &active_keys);

        // Alternate: remove oldest, insert new
        for i in 0..50 {
            let removed_key = keys[order[i]];
            tree.remove(order[i], &mut store).unwrap();
            active_keys.retain(|&k| k != removed_key);
            validate_tree(&tree, &store, &active_keys);

            tree.insert(order[50 + i], &mut store).unwrap();
            active_keys.push(keys[order[50 + i]]);
            active_keys.sort();
            validate_tree(&tree, &store, &active_keys);
        }
    }

    #[test]
    fn stress_test() {
        let keys: Vec<i32> = (0..500).collect();
        let mut store = NodeStore::new(&keys);
        let mut tree = RbTree::new();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        shuffle(&mut order);

        let mut seed = 0x6b8b_4567_9a1c_def0u64;
        let mut active_nodes = Vec::new();
        let mut available_nodes = order.clone();

        for _ in 0..10000 {
            let do_insert = if active_nodes.is_empty() {
                true
            } else if available_nodes.is_empty() {
                false
            } else {
                (lcg(&mut seed) % 10) < 7
            };

            if do_insert {
                let idx = (lcg(&mut seed) as usize) % available_nodes.len();
                let node_id = available_nodes.swap_remove(idx);
                tree.insert(node_id, &mut store).unwrap();
                active_nodes.push(node_id);
            } else {
                let idx = (lcg(&mut seed) as usize) % active_nodes.len();
                let node_id = active_nodes.swap_remove(idx);
                tree.remove(node_id, &mut store).unwrap();
                available_nodes.push(node_id);
            }

            let mut expected_keys: Vec<i32> = active_nodes.iter().map(|&id| keys[id]).collect();
            expected_keys.sort();
            validate_tree(&tree, &store, &expected_keys);
        }

        let mut expected_keys: Vec<i32> = active_nodes.iter().map(|&id| keys[id]).collect();
        expected_keys.sort();
        validate_tree(&tree, &store, &expected_keys);
    }
}

// END TESTING
