use core::marker::PhantomData;

use crate::error::Result;

use super::traits::{Get, GetMut};

#[allow(dead_code)]
pub struct List<Tag, T: Copy> {
    head: Option<T>,
    tail: Option<T>,
    len: usize,
    _tag: PhantomData<Tag>,
}

#[allow(dead_code)]
pub trait Linkable<Tag, T> {
    fn links(&self) -> &Links<Tag, T>;
    fn links_mut(&mut self) -> &mut Links<Tag, T>;
}

#[allow(dead_code)]
#[proc_macros::fmt]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Links<Tag, T> {
    prev: Option<T>,
    next: Option<T>,
    _tag: PhantomData<Tag>,
}

#[allow(dead_code)]
impl<Tag, T> Links<Tag, T> {
    pub const fn new() -> Self {
        Self {
            prev: None,
            next: None,
            _tag: PhantomData,
        }
    }
}

#[allow(dead_code)]
impl<Tag, T: Copy + PartialEq> List<Tag, T> {
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
            _tag: PhantomData,
        }
    }

    pub fn head(&self) -> Option<T> {
        self.head
    }

    pub fn tail(&self) -> Option<T> {
        self.tail
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push_front<S: Get<T> + GetMut<T>>(&mut self, id: T, storage: &mut S) -> Result<()>
    where
        <S as Get<T>>::Output: Linkable<Tag, T>,
    {
        self.detach_links(id, storage)?;

        match self.head {
            Some(old_head) => {
                let (new_node, old_head_node) = storage.get2_mut(id, old_head);
                let (new_node, old_head_node) = (new_node.ok_or(kerr!(NotFound))?, old_head_node.unwrap_or_else(|| {
                    bug!("node linked from list does not exist in storage.");
                }));

                new_node.links_mut().prev = None;
                new_node.links_mut().next = Some(old_head);

                old_head_node.links_mut().prev = Some(id);
            }
            None => {
                let new_node = storage.get_mut(id).ok_or(kerr!(NotFound))?;
                new_node.links_mut().prev = None;
                new_node.links_mut().next = None;
                self.tail = Some(id);
            }
        }

        self.head = Some(id);
        self.len += 1;
        Ok(())
    }

    /// Pushes `id` to the back of the list. If `id` is already in the list, it is moved to the back.
    /// 
    /// Errors if `id` does not exist in `storage` or if the node corresponding to `id` is linked but not in the list.
    pub fn push_back<S: Get<T> + GetMut<T>>(&mut self, id: T, storage: &mut S) -> Result<()>
    where
        <S as Get<T>>::Output: Linkable<Tag, T>,
    {
        self.detach_links(id, storage)?;

        match self.tail {
            Some(old_tail) => {
                let (new_node, old_tail_node) = storage.get2_mut(id, old_tail);
                let (new_node, old_tail_node) = (new_node.ok_or(kerr!(NotFound))?, old_tail_node.unwrap_or_else(|| {
                    bug!("node linked from list does not exist in storage.");
                }));

                new_node.links_mut().next = None;
                new_node.links_mut().prev = Some(old_tail);

                old_tail_node.links_mut().next = Some(id);
            }
            None => {
                let new_node = storage.get_mut(id).ok_or(kerr!(NotFound))?;
                new_node.links_mut().next = None;
                new_node.links_mut().prev = None;
                self.head = Some(id);
            }
        }

        self.tail = Some(id);
        self.len += 1;
        Ok(())
    }

    pub fn pop_front<S: Get<T> + GetMut<T>>(&mut self, storage: &mut S) -> Result<Option<T>>
    where
        <S as Get<T>>::Output: Linkable<Tag, T>,
    {
        let Some(id) = self.head else {
            return Ok(None);
        };

        self.remove(id, storage)?;
        Ok(Some(id))
    }

    pub fn pop_back<S: Get<T> + GetMut<T>>(&mut self, storage: &mut S) -> Result<Option<T>>
    where
        <S as Get<T>>::Output: Linkable<Tag, T>,
    {
        let Some(id) = self.tail else {
            return Ok(None);
        };

        self.remove(id, storage)?;
        Ok(Some(id))
    }

    /// Removes `id` from the list. Errors if `id` does not exist in `storage` or if the node corresponding to `id` is not linked.
    pub fn remove<S: Get<T> + GetMut<T>>(&mut self, id: T, storage: &mut S) -> Result<()>
    where
        <S as Get<T>>::Output: Linkable<Tag, T>,
    {
        let (prev, next, linked) = {
            let node = storage.get(id).ok_or(kerr!(NotFound))?;
            let links = node.links();
            let linked = self.head == Some(id)
                || self.tail == Some(id)
                || links.prev.is_some()
                || links.next.is_some();
            (links.prev, links.next, linked)
        };

        if !linked {
            return Err(kerr!(NotFound));
        }

        if let Some(prev_id) = prev {
            let prev_node = storage.get_mut(prev_id).unwrap_or_else(|| {
                bug!("node linked from list does not exist in storage.");
            });
            prev_node.links_mut().next = next;
        } else {
            self.head = next;
        }

        if let Some(next_id) = next {
            let next_node = storage.get_mut(next_id).unwrap_or_else(|| {
                bug!("node linked from list does not exist in storage.");
            });
            next_node.links_mut().prev = prev;
        } else {
            self.tail = prev;
        }

        let node = storage.get_mut(id).ok_or(kerr!(NotFound))?;
        node.links_mut().prev = None;
        node.links_mut().next = None;

        self.len = self.len.saturating_sub(1);
        Ok(())
    }

    /// Detaches `id` from any list it is currently in. If `id` is not in any list but is linked, the links are cleared.
    fn detach_links<S: Get<T> + GetMut<T>>(&mut self, id: T, storage: &mut S) -> Result<()>
    where
        <S as Get<T>>::Output: Linkable<Tag, T>,
    {
        let linked = {
            let node = storage.get(id).ok_or(kerr!(NotFound))?;
            let links = node.links();
            self.head == Some(id)
                || self.tail == Some(id)
                || links.prev.is_some()
                || links.next.is_some()
        };

        if linked {
            self.remove(id, storage)?;
        } else {
            let node = storage.get_mut(id).ok_or(kerr!(NotFound))?;
            node.links_mut().prev = None;
            node.links_mut().next = None;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::borrow::Borrow;

    use super::{Linkable, Links, List};
    use crate::types::{array::IndexMap, traits::{Get, ToIndex}};

    #[proc_macros::fmt]
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct Id(usize);

    impl ToIndex for Id {
        fn to_index<Q: Borrow<Self>>(idx: Option<Q>) -> usize {
            idx.as_ref().map_or(0, |k| k.borrow().0)
        }
    }

    #[derive(Clone, Copy)]
    struct TestTag;

    struct Node {
        links: Links<TestTag, Id>,
    }

    impl Node {
        fn new() -> Self {
            Self {
                links: Links::new(),
            }
        }
    }

    impl Linkable<TestTag, Id> for Node {
        fn links(&self) -> &Links<TestTag, Id> {
            &self.links
        }

        fn links_mut(&mut self) -> &mut Links<TestTag, Id> {
            &mut self.links
        }
    }

    fn storage() -> IndexMap<Id, Node, 8> {
        let mut map = IndexMap::new();
        for i in 0..4 {
            assert!(map.insert(&Id(i), Node::new()).is_ok());
        }
        map
    }

    #[test]
    fn push_front_and_remove() {
        let mut s = storage();
        let mut list = List::<TestTag, Id>::new();

        list.push_front(Id(1), &mut s).unwrap();
        list.push_front(Id(2), &mut s).unwrap();
        list.push_front(Id(3), &mut s).unwrap();

        assert_eq!(list.head(), Some(Id(3)));
        assert_eq!(list.tail(), Some(Id(1)));
        assert_eq!(list.len(), 3);

        list.remove(Id(2), &mut s).unwrap();
        assert_eq!(list.head(), Some(Id(3)));
        assert_eq!(list.tail(), Some(Id(1)));
        assert_eq!(list.len(), 2);

        let n3 = s.get(Id(3)).unwrap();
        let n1 = s.get(Id(1)).unwrap();
        assert_eq!(n3.links().next, Some(Id(1)));
        assert_eq!(n1.links().prev, Some(Id(3)));
    }

    #[test]
    fn push_back_and_remove() {
        let mut s = storage();
        let mut list = List::<TestTag, Id>::new();

        list.push_back(Id(1), &mut s).unwrap();
        list.remove(Id(1), &mut s);

        assert_eq!(list.head(), None);
        assert_eq!(list.tail(), None);
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn push_back_same_id_reinserts() {
        let mut s = storage();
        let mut list = List::<TestTag, Id>::new();

        list.push_back(Id(1), &mut s).unwrap();
        list.push_back(Id(1), &mut s).unwrap();

        assert_eq!(list.head(), Some(Id(1)));
        assert_eq!(list.tail(), Some(Id(1)));
        assert_eq!(list.len(), 1);

        let n1 = s.get(Id(1)).unwrap();
        assert_eq!(n1.links().prev, None);
        assert_eq!(n1.links().next, None);
    }

    #[test]
    fn pop_back_ordered() {
        let mut s = storage();
        let mut list = List::<TestTag, Id>::new();

        list.push_back(Id(1), &mut s).unwrap();
        list.push_back(Id(2), &mut s).unwrap();
        list.push_back(Id(3), &mut s).unwrap();

        assert_eq!(list.pop_back(&mut s).unwrap(), Some(Id(3)));
        assert_eq!(list.pop_back(&mut s).unwrap(), Some(Id(2)));
        assert_eq!(list.pop_back(&mut s).unwrap(), Some(Id(1)));
        assert_eq!(list.pop_back(&mut s).unwrap(), None);
        assert!(list.is_empty());
    }

    #[test]
    fn pop_front_ordered() {
        let mut s = storage();
        let mut list = List::<TestTag, Id>::new();

        list.push_back(Id(1), &mut s).unwrap();
        list.push_back(Id(2), &mut s).unwrap();
        list.push_back(Id(3), &mut s).unwrap();

        assert_eq!(list.pop_front(&mut s).unwrap(), Some(Id(1)));
        assert_eq!(list.pop_front(&mut s).unwrap(), Some(Id(2)));
        assert_eq!(list.pop_front(&mut s).unwrap(), Some(Id(3)));
        assert_eq!(list.pop_front(&mut s).unwrap(), None);
        assert!(list.is_empty());
    }
}