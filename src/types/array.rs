//! This module implements static and dynamic arrays for in-kernel use.

use crate::error::Result;

use super::{
    traits::{Get, GetMut, ToIndex},
    boxed::Box,
};

use core::{borrow::Borrow, mem::MaybeUninit};
use core::{
    ops::{Index, IndexMut},
};

/// This is a fixed-size map that can store up to N consecutive elements.
#[derive(Debug)]
pub struct IndexMap<K: ?Sized + ToIndex, V, const N: usize>
{
    data: [Option<V>; N],
    phantom: core::marker::PhantomData<K>,
}

#[allow(dead_code)]
impl<K: ?Sized + ToIndex, V, const N: usize> IndexMap<K, V, N>
{
    /// Create a new IndexMap.
    ///
    /// Returns a new IndexMap.
    pub const fn new() -> Self {
        Self {
            data: [const { None }; N],
            phantom: core::marker::PhantomData,
        }
    }

    /// Insert a value at the given index.
    ///
    /// `index` - The index to insert the value at.
    /// `value` - The value to insert.
    ///
    /// Returns `Ok(())` if the index was in-bounds, otherwise `Err(KernelError::OutOfMemory)`.
    pub fn insert(&mut self, idx: &K, value: V) -> Result<()> {
        let idx = K::to_index(Some(idx));

        if idx < N {
            self.data[idx] = Some(value);
            Ok(())
        } else {
            Err(kerr!(OutOfMemory))
        }
    }

    /// Insert a value at the next available index.
    ///
    /// `value` - The value to insert.
    ///
    /// Returns `Ok(index)` if the value was inserted, otherwise `Err(KernelError::OutOfMemory)`.
    pub fn insert_next(&mut self, value: V) -> Result<usize> {
        for (i, slot) in self.data.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(value);
                return Ok(i);
            }
        }

        Err(kerr!(OutOfMemory))
    }

    /// Remove the value at the given index.
    ///
    /// `index` - The index to remove the value from.
    ///
    /// Returns the value if it was removed, otherwise `None`.
    pub fn remove(&mut self, idx: &K) -> Option<V> {
        let idx = K::to_index(Some(idx));

        if idx < N {
            self.data[idx].take()
        } else {
            None
        }
    }

    /// Get an iterator over the elements in the map.
    ///
    /// Returns an iterator over the elements in the map.
    pub fn iter(&self) -> impl Iterator<Item = &Option<V>> {
        self.data.iter()
    }

    /// Get an cycling iterator over the elements in the map, starting from the given index.
    ///
    /// `index` - The index to start the iterator from.
    ///
    /// Returns an iterator over the elements in the map.
    pub fn iter_from_cycle(&self, idx: Option<&K>) -> impl Iterator<Item = &Option<V>> {
        self.data.iter().cycle().skip(K::to_index(idx) + 1)
    }

    /// Get the next index that contains a value (this will cycle).
    ///
    /// `index` - The index to start the search from.
    ///
    /// Returns the next index (potentially < index) that contains a value, otherwise `None`.
    pub fn next(&self, idx: Option<&K>) -> Option<usize> {
        for (i, elem) in self.iter_from_cycle(idx).enumerate() {
            if elem.is_some() {
                let idx = K::to_index(idx);
                return Some((idx + i + 1) % N);
            }
        }

        None
    }

    pub fn at_cont(&self, idx: usize) -> Option<&V> {
        if idx < N {
            self.data[idx].as_ref()
        } else {
            None
        }
    }

    pub fn find_empty(&self) -> Option<usize> {
        for (i, slot) in self.data.iter().enumerate() {
            if slot.is_none() {
                return Some(i);
            }
        }

        None
    }
}

impl<K: Copy + ToIndex, V, const N: usize> Index<K> for IndexMap<K, V, N>
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(&index).unwrap()
    }
}

impl<K: Copy + ToIndex, V, const N: usize> IndexMut<K> for IndexMap<K, V, N>
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(&index).unwrap()
    }
}

impl<K: ?Sized + ToIndex, V, const N: usize> Get<K> for IndexMap<K, V, N>
{
    type Output = V;

    fn get<Q: Borrow<K>>(&self, index: Q) -> Option<&Self::Output> {
        let idx = K::to_index(Some(index.borrow()));
        if idx < N {
            self.data[idx].as_ref()
        } else {
            None
        }
    }
}

impl<K: ?Sized + ToIndex, V, const N: usize> GetMut<K> for IndexMap<K, V, N> {
    fn get_mut<Q: Borrow<K>>(&mut self, index: Q) -> Option<&mut Self::Output> {
        let idx = K::to_index(Some(index.borrow()));
        if idx < N {
            self.data[idx].as_mut()
        } else {
            None
        }
    }

    fn get2_mut<Q: Borrow<K>>(&mut self, index1: Q, index2: Q) -> (Option<&mut Self::Output>, Option<&mut Self::Output>) {
        let index1 = K::to_index(Some(index1.borrow()));
        let index2 = K::to_index(Some(index2.borrow()));

        if index1 == index2 {
            debug_assert!(false, "get2_mut called with identical indices");
            return (None, None);
        }

        let (left, right) = self.data.split_at_mut(index1.max(index2));

        if index1 < index2 {
            let elem1 = left[index1].as_mut();
            let elem2 = right[0].as_mut();
            (elem1, elem2)
        } else {
            let elem1 = right[0].as_mut();
            let elem2 = left[index2].as_mut();
            (elem1, elem2)
        }
    }

    fn get3_mut<Q: Borrow<K>>(
        &mut self,
        index1: Q,
        index2: Q,
        index3: Q,
    ) -> (Option<&mut Self::Output>, Option<&mut Self::Output>, Option<&mut Self::Output>) {
        let index1 = K::to_index(Some(index1.borrow()));
        let index2 = K::to_index(Some(index2.borrow()));
        let index3 = K::to_index(Some(index3.borrow()));

        if index1 == index2 || index1 == index3 || index2 == index3 {
            debug_assert!(false, "get3_mut called with identical indices");
            return (None, None, None);
        }

        let ptr1 = &mut self.data[index1] as *mut Option<V>;
        let ptr2 = &mut self.data[index2] as *mut Option<V>;
        let ptr3 = &mut self.data[index3] as *mut Option<V>;

        // Safety: the elements at index1, index2 and index3 are nowhere else borrowed mutably by function contract.
        // And they are disjoint because of the check above.
        unsafe { ((*ptr1).as_mut(), (*ptr2).as_mut(), (*ptr3).as_mut()) }
    }
}

/// This is a vector that can store up to N elements inline and will allocate on the heap if more are needed.
#[derive(Debug)]
pub struct Vec<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
    extra: Box<[MaybeUninit<T>]>,
}

#[allow(dead_code)]
impl<T: Clone + Copy, const N: usize> Vec<T, N> {
    /// Create a new Vec.
    ///
    /// Returns a new Vec.
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [const { MaybeUninit::uninit() }; N],
            extra: Box::new_slice_empty(),
        }
    }

    /// Reserve additional space in the Vec.
    ///
    /// `additional` - The additional space to reserve.
    ///
    /// Returns `Ok(())` if the space was reserved, otherwise `Err(KernelError::OutOfMemory)`.
    pub fn reserve(&mut self, additional: usize) -> Result<()> {
        let len_extra = self.extra.len();

        // Check if we have enough space in the inline storage.
        if self.len + additional <= N + len_extra {
            return Ok(());
        }

        // If we don't have enough space, we need to grow the extra storage.
        let grow = additional - N + len_extra;
        let mut new_extra = Box::new_slice_uninit(grow)?;

        // Check that the new extra storage has the requested length.
        bug_on!(new_extra.len() != grow);

        // Copy the old extra storage into the new one.
        new_extra[..len_extra].copy_from_slice(&self.extra);

        // Replace the old extra storage with the new one. The old one will be dropped.
        self.extra = new_extra;
        Ok(())
    }

    /// Reserve a fixed amount of space in the Vec. Does nothing if enough space is present already.
    ///
    /// `total_capacity` - The total space to be reserved.
    ///
    /// Returns `Ok(())` if the space was reserved, otherwise `Err(KernelError::OutOfMemory)`.
    pub fn reserve_total_capacity(&mut self, total_capacity: usize) -> Result<()> {
        // Check if we already have enough space
        if self.capacity() >= total_capacity {
            return Ok(());
        }

        // If we don't have enough space, we need to grow the extra storage.
        let new_out_of_line_cap = total_capacity - N;
        let mut new_extra = Box::new_slice_uninit(new_out_of_line_cap)?;

        // Check that the new extra storage has the requested length.
        bug_on!(new_extra.len() != new_out_of_line_cap);

        let curr_out_of_line_size = self.extra.len();
        // Copy the old extra storage into the new one.
        new_extra[..curr_out_of_line_size].copy_from_slice(&self.extra);

        // Replace the old extra storage with the new one. The old one will be dropped.
        self.extra = new_extra;
        Ok(())
    }

    /// Create a new Vec with the given length and value.
    ///
    /// `length` - The length of the Vec.
    /// `value` - The value to initialize the elements in the Vec with.
    ///
    /// Returns the new Vec or `Err(KernelError::OutOfMemory)` if the allocation failed.
    pub fn new_init(length: usize, value: T) -> Result<Self> {
        let mut vec = Self::new();

        // Check if we can fit all elements in the inline storage.
        if length <= N {
            // Initialize all elements in the inline storage.
            for i in 0..length {
                vec.data[i].write(value);
            }
        } else {
            // Initialize all elements in the inline storage.
            vec.data.fill(MaybeUninit::new(value));

            // Check if we need to allocate extra storage.
            if length - N > 0 {
                // Allocate extra storage for the remaining elements.
                let mut extra = Box::new_slice_uninit(length - N)?;

                // Initialize all the required elements in the extra storage.
                for i in N..length {
                    extra[i - N].write(value);
                }

                // Set the extra storage in the Vec.
                vec.extra = extra;
            }
        }

        Ok(vec)
    }

    /// Push a value onto the Vec.
    ///
    /// `value` - The value to push.
    ///
    /// Returns `Ok(())` if the value was pushed, otherwise `Err(KernelError::OutOfMemory)`.
    pub fn push(&mut self, value: T) -> Result<()> {
        // Check if we have enough space in the inline storage.
        if self.len < N {
            // Push the value into the inline storage.
            self.data[self.len].write(value);
            self.len += 1;
            Ok(())
        } else {
            let len_extra = self.extra.len();

            // Check if we have enough space in the extra storage.
            if self.len < N + len_extra {
                // Push the value into the extra storage.
                self.extra[self.len - N].write(value);
                self.len += 1;
                Ok(())
            } else {
                // We need to grow the extra storage.
                let grow = (len_extra + 1) * 2;
                let mut new_extra = Box::new_slice_uninit(grow)?;

                bug_on!(new_extra.len() != grow);

                // Copy the old extra storage into the new one.
                new_extra[..len_extra].copy_from_slice(&self.extra);

                // Replace the old extra storage with the new one. The old one will be dropped.
                self.extra = new_extra;
                self.extra[len_extra].write(value);
                self.len += 1;
                Ok(())
            }
        }
    }

    /// Pop a value from the Vec.
    ///
    /// Returns the value if it was popped, otherwise `None`.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.remove(self.len - 1)
    }

    /// Remove the value at the given index.
    ///
    /// `index` - The index to remove the value from.
    ///
    /// Returns the value if it was removed, otherwise `None`.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        // Check if the index is in-bounds.
        if index >= self.len {
            return None;
        }

        // Get the value at the given index.
        let value = self.at(index).cloned();

        // Check if we need to move inline storage elements.
        if index < N {
            // Move the elements in the inline storage.
            let end = core::cmp::min(self.len, N);

            // Safety: index is less than N and min too.
            self.data.copy_within(index + 1..end, index);

            // Check if we need to move the first extra storage element into the inline storage.
            if let Some(value) = self.at(N) {
                self.data[end - 1].write(*value);
            }

            // Move the elements in the extra storage.
            if self.len() > N {
                self.extra.copy_within(1..self.len - N, 0);
            }
        } else {
            // We only need to move the elements in the extra storage.

            let index = index - N;
            let end = self.len - N;

            // Safety: index is less than N and min too.
            self.extra.copy_within(index + 1..end, index);
        }

        self.len -= 1;
        value
    }

    /// Get the length of the Vec.
    ///
    /// Returns the length of the Vec.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Get the value at the given index.
    ///
    /// `index` - The index to get the value from.
    ///
    /// Returns `Some(&T)` if the index is in-bounds, otherwise `None`.
    pub fn at(&self, index: usize) -> Option<&T> {
        // Check if the index is in-bounds.
        if index > self.len - 1 {
            return None;
        }

        if index < N {
            // Safety: the elements until self.len are initialized.
            unsafe { Some(self.data[index].assume_init_ref()) }
        } else {
            let index = index - N;
            // Safety: the elements until self.len - N are initialized.
            unsafe { Some(self.extra[index].assume_init_ref()) }
        }
    }

    /// Get a mutable reference to the value at the given index.
    ///
    /// `index` - The index to get the value from.
    ///
    /// Returns `Some(&mut T)` if the index is in-bounds, otherwise `None`.
    pub fn at_mut(&mut self, index: usize) -> Option<&mut T> {
        // Check if the index is in-bounds.
        if index > self.len - 1 {
            return None;
        }

        if index < N {
            // Safety: the elements until self.len are initialized.
            unsafe { Some(self.data[index].assume_init_mut()) }
        } else {
            let index = index - N;
            // Safety: the elements until self.len - N are initialized.
            unsafe { Some(self.extra[index].assume_init_mut()) }
        }
    }

    fn at_mut_unchecked(&mut self, index: usize) -> *mut T {
        if index < N {
            // Safety: the elements until self.len are initialized.
            // The element at index is nowhere else borrowed mutably by function contract.
            self.data[index].as_mut_ptr()
        } else {
            let index = index - N;
            // Safety: the elements until self.len - N are initialized.
            // The element at index is nowhere else borrowed mutably by function contract.
            self.extra[index].as_mut_ptr()
        }
    }

    /// Get disjoint mutable references to the values at the given indices.
    ///
    /// `index1` - The first index.
    /// `index2` - The second index.
    ///
    /// Returns `Some(&mut T, &mut T)` if the indices are in-bounds and disjoint, otherwise `None`.
    pub fn at2_mut(&mut self, index1: usize, index2: usize) -> (Option<&mut T>, Option<&mut T>) {
        if index1 == index2 {
            debug_assert!(false, "at2_mut called with identical indices");
            return (None, None);
        }

        let ptr1 = self.at_mut_unchecked(index1);
        let ptr2 = self.at_mut_unchecked(index2);

        // Safety: the elements at index1 and index2 are nowhere else borrowed mutably by function contract.
        // And they are disjoint because of the check above.
        unsafe { (Some(&mut *ptr1), Some(&mut *ptr2)) }
    }

    /// Get disjoint mutable references to the values at the given indices.
    ///
    /// `index1` - The first index.
    /// `index2` - The second index.
    /// `index3` - The third index.
    ///
    /// Returns `Some(&mut T, &mut T, &mut T)` if the indices are in-bounds and disjoint, otherwise `None`.
    pub fn at3_mut(
        &mut self,
        index1: usize,
        index2: usize,
        index3: usize,
    ) -> (Option<&mut T>, Option<&mut T>, Option<&mut T>) {
        if index1 == index2 || index1 == index3 || index2 == index3 {
            debug_assert!(false, "at3_mut called with identical indices");
            return (None, None, None);
        }

        let ptr1 = self.at_mut_unchecked(index1);
        let ptr2 = self.at_mut_unchecked(index2);
        let ptr3 = self.at_mut_unchecked(index3);

        // Safety: the elements at index1, index2 and index3 are nowhere else borrowed mutably by function contract.
        // And they are disjoint because of the check above.
        unsafe { (Some(&mut *ptr1), Some(&mut *ptr2), Some(&mut *ptr3)) }
    }

    /// Swap the values at the given indices.
    ///
    /// `a` - The first index.
    /// `b` - The second index.
    pub fn swap(&mut self, a: usize, b: usize) {
        // Check if the indices are in-bounds.
        if a >= self.len || b >= self.len {
            return;
        }

        if a < N && b < N {
            // Both indices are in the inline storage.
            self.data.swap(a, b);
        } else if a >= N && b >= N {
            // Both indices are in the extra storage.
            self.extra.swap(a - N, b - N);
        } else if a >= N {
            // The first index is in the extra storage.
            core::mem::swap(&mut self.extra[a - N], &mut self.data[b]);
        } else {
            // The second index is in the extra storage.
            core::mem::swap(&mut self.data[a], &mut self.extra[b - N]);
        }
    }

    /// Check if the Vec is empty.
    ///
    /// Returns `true` if the Vec is empty, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get total amount of space in Vec (in- and out-of-line)
    ///
    /// Returns total amount of  reserved space in the vec
    pub fn capacity(&self) -> usize {
        N + self.extra.len()
    }
}

impl<T, const N: usize> Drop for Vec<T, N> {
    fn drop(&mut self) {
        let min = core::cmp::min(self.len, N);

        // Drop all elements in the inline storage.
        for elem in &mut self.data[0..min] {
            // Safety: the elements until min are initialized.
            unsafe {
                elem.assume_init_drop();
            }
        }

        // Drop all elements in the extra storage.
        for elem in &mut (*self.extra)[0..self.len - N] {
            // Safety: the elements until self.len - N are initialized.
            unsafe {
                elem.assume_init_drop();
            }
        }
    }
}

impl<T: Clone + Copy, const N: usize> Index<usize> for Vec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.at(index).unwrap()
    }
}

impl<T: Clone + Copy, const N: usize> IndexMut<usize> for Vec<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.at_mut(index).unwrap()
    }
}

impl<T: Clone + Copy, const N: usize> Get<usize> for Vec<T, N> {
    type Output = T;

    fn get<Q: Borrow<usize>>(&self, index: Q) -> Option<&Self::Output> {
        self.at(*index.borrow())
    }
}

impl<T: Clone + Copy, const N: usize> GetMut<usize> for Vec<T, N> {
    fn get_mut<Q: Borrow<usize>>(&mut self, index: Q) -> Option<&mut Self::Output> {
        self.at_mut(*index.borrow())
    }

    fn get2_mut<Q: Borrow<usize>>(
        &mut self,
        index1: Q,
        index2: Q,
    ) -> (Option<&mut Self::Output>, Option<&mut Self::Output>) {
        self.at2_mut(*index1.borrow(), *index2.borrow())
    }

    fn get3_mut<Q: Borrow<usize>>(
        &mut self,
        index1: Q,
        index2: Q,
        index3: Q,
    ) -> (Option<&mut Self::Output>, Option<&mut Self::Output>, Option<&mut Self::Output>) {
        self.at3_mut(*index1.borrow(), *index2.borrow(), *index3.borrow())
    }
}
