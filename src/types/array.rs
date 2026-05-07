//! This module implements static and dynamic arrays for in-kernel use.

use crate::{error::Result, types::bitset::BitAlloc};

use super::{
    boxed::Box,
    traits::{Get, GetMut, ToIndex},
};

use core::mem::ManuallyDrop;
use core::ops::{Index, IndexMut};
use core::{borrow::Borrow, mem::MaybeUninit};

/// This is a fixed-size map that can store up to N consecutive elements.
#[proc_macros::fmt]
pub struct IndexMap<K: ?Sized + ToIndex, V, const N: usize> {
    data: [Option<V>; N],
    phantom: core::marker::PhantomData<K>,
}

#[allow(dead_code)]
impl<K: ?Sized + ToIndex, V, const N: usize> IndexMap<K, V, N> {
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
    /// Returns `Ok(())` if the index was in-bounds, otherwise `Err(KernelError::ENOMEM)`.
    pub fn insert(&mut self, idx: &K, value: V) -> Result<()> {
        let idx = K::to_index(Some(idx));

        if idx < N {
            self.data[idx] = Some(value);
            Ok(())
        } else {
            Err(kerr!(ENOMEM))
        }
    }

    /// Remove the value at the given index.
    ///
    /// `index` - The index to remove the value from.
    ///
    /// Returns the value if it was removed, otherwise `None`.
    pub fn remove(&mut self, idx: &K) -> Option<V> {
        let idx = K::to_index(Some(idx));

        if idx < N { self.data[idx].take() } else { None }
    }

    pub fn raw_insert(&mut self, idx: usize, value: V) -> Result<()> {
        if idx < N {
            self.data[idx] = Some(value);
            Ok(())
        } else {
            Err(kerr!(ENOMEM))
        }
    }

    pub fn raw_remove(&mut self, idx: usize) -> Option<V> {
        if idx < N { self.data[idx].take() } else { None }
    }

    pub fn raw_at(&self, idx: usize) -> Option<&V> {
        if idx < N {
            self.data[idx].as_ref()
        } else {
            None
        }
    }

    pub fn raw_at_mut(&mut self, idx: usize) -> Option<&mut V> {
        if idx < N {
            self.data[idx].as_mut()
        } else {
            None
        }
    }
}

impl<K: Copy + ToIndex, V, const N: usize> Index<K> for IndexMap<K, V, N> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get::<K>(index).unwrap()
    }
}

impl<K: Copy + ToIndex, V, const N: usize> IndexMut<K> for IndexMap<K, V, N> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut::<K>(index).unwrap()
    }
}

impl<K: ?Sized + ToIndex, V, const N: usize> Get<K> for IndexMap<K, V, N> {
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

    fn get2_mut<Q: Borrow<K>>(
        &mut self,
        index1: Q,
        index2: Q,
    ) -> (Option<&mut Self::Output>, Option<&mut Self::Output>) {
        let idx1 = K::to_index(Some(index1.borrow()));
        let idx2 = K::to_index(Some(index2.borrow()));

        if idx1 == idx2 {
            debug_assert!(false, "get2_mut called with identical indices");
            return (None, None);
        }

        if idx1 >= N {
            return (
                None,
                if idx2 < N {
                    self.data[idx2].as_mut()
                } else {
                    None
                },
            );
        }
        if idx2 >= N {
            return (self.data[idx1].as_mut(), None);
        }

        let (left, right) = self.data.split_at_mut(idx1.max(idx2));

        if idx1 < idx2 {
            let elem1 = left[idx1].as_mut();
            let elem2 = right[0].as_mut();
            (elem1, elem2)
        } else {
            let elem1 = right[0].as_mut();
            let elem2 = left[idx2].as_mut();
            (elem1, elem2)
        }
    }

    fn get3_mut<Q: Borrow<K>>(
        &mut self,
        index1: Q,
        index2: Q,
        index3: Q,
    ) -> (
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
    ) {
        let idx1 = K::to_index(Some(index1.borrow()));
        let idx2 = K::to_index(Some(index2.borrow()));
        let idx3 = K::to_index(Some(index3.borrow()));

        if idx1 == idx2 || idx1 == idx3 || idx2 == idx3 {
            debug_assert!(false, "get3_mut called with identical indices");
            return (None, None, None);
        }

        let ptr1 = if idx1 < N {
            Some(&mut self.data[idx1] as *mut Option<V>)
        } else {
            None
        };
        let ptr2 = if idx2 < N {
            Some(&mut self.data[idx2] as *mut Option<V>)
        } else {
            None
        };
        let ptr3 = if idx3 < N {
            Some(&mut self.data[idx3] as *mut Option<V>)
        } else {
            None
        };

        // Safety: each pointer comes from an in-bounds slot in self.data, and the three indices
        // are pairwise distinct (check above), so the resulting references are disjoint.
        unsafe {
            (
                ptr1.and_then(|p| (*p).as_mut()),
                ptr2.and_then(|p| (*p).as_mut()),
                ptr3.and_then(|p| (*p).as_mut()),
            )
        }
    }
}

/// This is a vector that can store up to N elements inline and will allocate on the heap if more are needed.
#[proc_macros::fmt]
pub struct Vec<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
    extra: Box<[MaybeUninit<T>]>,
}

#[allow(dead_code)]
impl<T, const N: usize> Vec<T, N> {
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

    pub const fn from_array(arr: [T; N]) -> Self {
        let arr = ManuallyDrop::new(arr);
        let mut data = [const { MaybeUninit::uninit() }; N];

        let src: *const [T; N] = &arr as *const ManuallyDrop<[T; N]> as *const [T; N];
        let dst: *mut [MaybeUninit<T>; N] = &mut data;

        let mut i = 0;
        while i < N {
            unsafe {
                let value = core::ptr::read((src as *const T).add(i));
                (dst as *mut MaybeUninit<T>)
                    .add(i)
                    .write(MaybeUninit::new(value));
            }
            i += 1;
        }

        Self {
            len: N,
            data,
            extra: Box::new_slice_empty(),
        }
    }

    unsafe fn move_from_slice_to_uninit(dst: &mut [MaybeUninit<T>], src: &[MaybeUninit<T>]) {
        assert_eq!(dst.len(), src.len());

        unsafe {
            for i in 0..dst.len() {
                dst[i].write(src[i].as_ptr().read());
            }
        }
    }

    unsafe fn move_within_to_uninit(
        slice: &mut [MaybeUninit<T>],
        src_index: usize,
        dst_index: usize,
        count: usize,
    ) {
        assert!(src_index + count <= slice.len());
        assert!(dst_index + count <= slice.len());

        unsafe {
            for i in 0..count {
                let value = slice[src_index + i].as_ptr().read();
                slice[dst_index + i].write(value);
            }
        }
    }

    /// Reserve additional space in the Vec.
    ///
    /// `additional` - The additional space to reserve.
    ///
    /// Returns `Ok(())` if the space was reserved, otherwise `Err(KernelError::ENOMEM)`.
    pub fn reserve(&mut self, additional: usize) -> Result<()> {
        let len_extra = self.extra.len();

        // Check if we have enough space in the inline storage.
        if self.len + additional <= N + len_extra {
            return Ok(());
        }

        // If we don't have enough space, we need to grow the extra storage.
        let grow = self.len + additional - N;
        let mut new_extra = Box::new_slice_uninit(grow)?;

        // Check that the new extra storage has the requested length.
        bug_on!(new_extra.len() != grow);

        let len_initialized_extra = self.len.saturating_sub(N);

        // Move the old extra storage into the new one.
        unsafe {
            Self::move_from_slice_to_uninit(
                &mut new_extra[..len_initialized_extra],
                &self.extra[..len_initialized_extra],
            );
        }

        // Replace the old extra storage with the new one. The old one will be dropped.
        self.extra = new_extra;
        Ok(())
    }

    /// Reserve a fixed amount of space in the Vec. Does nothing if enough space is present already.
    ///
    /// `total_capacity` - The total space to be reserved.
    ///
    /// Returns `Ok(())` if the space was reserved, otherwise `Err(KernelError::ENOMEM)`.
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

        let curr_out_of_line_size = self.len.saturating_sub(N);
        // Move the old extra storage into the new one.
        unsafe {
            Self::move_from_slice_to_uninit(
                &mut new_extra[..curr_out_of_line_size],
                &self.extra[..curr_out_of_line_size],
            );
        }

        // Replace the old extra storage with the new one. The old one will be dropped.
        self.extra = new_extra;
        Ok(())
    }

    /// Create a new Vec with the given length and value.
    ///
    /// `length` - The length of the Vec.
    /// `value` - The value to initialize the elements in the Vec with.
    ///
    /// Returns the new Vec or `Err(KernelError::ENOMEM)` if the allocation failed.
    pub fn new_init(length: usize, value: T) -> Result<Self>
    where
        T: Clone,
    {
        let mut vec = Self::new();

        // Check if we can fit all elements in the inline storage.
        if length <= N {
            // Initialize all elements in the inline storage.
            for i in 0..length {
                vec.data[i].write(value.clone());
                // Bump len after each write so a panicking T::clone leaves Drop able
                // to clean up the slots that were already initialized.
                vec.len = i + 1;
            }
        } else {
            // Allocate the extra storage first and install it on `vec` so any later
            // failure (clone panic) leaves Drop with a consistent (len, extra) view,
            // and so an OOM here cannot leak inline clones (none have been written yet).
            let extra_len = length - N;
            vec.extra = Box::new_slice_uninit(extra_len)?;

            for elem in &mut vec.data {
                elem.write(value.clone());
                vec.len += 1;
            }

            for i in 0..extra_len {
                vec.extra[i].write(value.clone());
                vec.len += 1;
            }
        }

        vec.len = length;
        Ok(vec)
    }

    /// Push a value onto the Vec.
    ///
    /// `value` - The value to push.
    ///
    /// Returns `Ok(())` if the value was pushed, otherwise `Err(KernelError::ENOMEM)`.
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

                let len_initialized_extra = self.len - N;

                // Move the old extra storage into the new one.
                unsafe {
                    Self::move_from_slice_to_uninit(
                        &mut new_extra[..len_initialized_extra],
                        &self.extra[..len_initialized_extra],
                    );
                }

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
        let value = unsafe { self.at_mut_unchecked(index).read() };

        // Check if we need to move inline storage elements.
        if index < N {
            // Move the elements in the inline storage.
            let end = core::cmp::min(self.len, N);

            // Safety: index is less than N and min too.
            unsafe {
                Self::move_within_to_uninit(&mut self.data, index + 1, index, end - index - 1);
            }

            // Check if we need to move the first extra storage element into the inline storage.
            if self.len() > N {
                let value = unsafe { self.extra[0].as_ptr().read() };
                self.data[end - 1].write(value);
            }

            // Move the elements in the extra storage.
            if self.len() > N {
                unsafe {
                    Self::move_within_to_uninit(&mut self.extra, 1, 0, self.len - N - 1);
                }
            }
        } else {
            // We only need to move the elements in the extra storage.

            let index = index - N;
            let end = self.len - N;

            // Safety: index is less than N and min too.
            unsafe {
                Self::move_within_to_uninit(&mut self.extra, index + 1, index, end - index - 1);
            }
        }

        self.len -= 1;
        Some(value)
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
        if index >= self.len {
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
        if index >= self.len {
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

        let in1 = index1 < self.len;
        let in2 = index2 < self.len;
        let ptr1 = if in1 {
            Some(self.at_mut_unchecked(index1))
        } else {
            None
        };
        let ptr2 = if in2 {
            Some(self.at_mut_unchecked(index2))
        } else {
            None
        };

        // Safety: each pointer is only constructed when the corresponding index is < self.len,
        // so it points to an initialized slot. The two indices are pairwise distinct (check
        // above), so the resulting references are disjoint.
        unsafe { (ptr1.map(|p| &mut *p), ptr2.map(|p| &mut *p)) }
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

        let in1 = index1 < self.len;
        let in2 = index2 < self.len;
        let in3 = index3 < self.len;
        let ptr1 = if in1 {
            Some(self.at_mut_unchecked(index1))
        } else {
            None
        };
        let ptr2 = if in2 {
            Some(self.at_mut_unchecked(index2))
        } else {
            None
        };
        let ptr3 = if in3 {
            Some(self.at_mut_unchecked(index3))
        } else {
            None
        };

        // Safety: each pointer is only constructed when the corresponding index is < self.len,
        // so it points to an initialized slot. The three indices are pairwise distinct (check
        // above), so the resulting references are disjoint.
        unsafe {
            (
                ptr1.map(|p| &mut *p),
                ptr2.map(|p| &mut *p),
                ptr3.map(|p| &mut *p),
            )
        }
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

impl<T, const N: usize> Vec<T, N> {
    /// Clear the Vec, dropping all elements.
    pub fn clear(&mut self) {
        let min = core::cmp::min(self.len, N);

        // Drop all elements in the inline storage.
        for elem in &mut self.data[0..min] {
            // Safety: the elements until min are initialized.
            unsafe {
                elem.assume_init_drop();
            }
        }

        // Drop all elements in the extra storage.
        for elem in &mut (*self.extra)[0..self.len - min] {
            // Safety: the elements until self.len - N are initialized.
            unsafe {
                elem.assume_init_drop();
            }
        }

        self.len = 0;
    }
}

impl<T, const N: usize> Drop for Vec<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const N: usize> Clone for Vec<T, N>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut new_vec = Self::new();
        let min = core::cmp::min(self.len, N);

        bug_on!(new_vec.reserve_total_capacity(self.len).is_err());

        // Clone the elements in the inline storage.
        for i in 0..min {
            // Safety: the elements until self.len are initialized.
            let value = unsafe { self.data[i].assume_init_ref() };
            new_vec.data[i].write(value.clone());
        }

        // Clone the elements in the extra storage.
        for i in 0..self.len - min {
            // Safety: the elements until self.len - N are initialized.
            let value = unsafe { self.extra[i].assume_init_ref() };
            new_vec.extra[i].write(value.clone());
        }

        new_vec.len = self.len;
        new_vec
    }
}

impl<T, const N: usize> Index<usize> for Vec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.at(index).unwrap()
    }
}

impl<T, const N: usize> IndexMut<usize> for Vec<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.at_mut(index).unwrap()
    }
}

impl<T, const N: usize> Get<usize> for Vec<T, N> {
    type Output = T;

    fn get<Q: Borrow<usize>>(&self, index: Q) -> Option<&Self::Output> {
        self.at(*index.borrow())
    }
}

impl<T, const N: usize> GetMut<usize> for Vec<T, N> {
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
    ) -> (
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
    ) {
        self.at3_mut(*index1.borrow(), *index2.borrow(), *index3.borrow())
    }
}

/// This is an IndexMap that additionally tracks which indices are occupied through a bitset.
/// WORDS is the number of usize words needed to track N entries, you should set it to WORDS = N.div_ceil(usize::BITS as usize).
/// Its currently impossible to set this value automatically because of const generic limitations.
pub struct BitReclaimMap<K: ?Sized + ToIndex, V, const N: usize> {
    map: IndexMap<K, V, N>,
    free: BitAlloc<N>,
}

impl<K: ?Sized + ToIndex, V, const N: usize> BitReclaimMap<K, V, N> {
    pub const fn new() -> Self {
        Self {
            map: IndexMap::new(),
            free: BitAlloc::from_array([!0usize; N]),
        }
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, value: V) -> Result<usize> {
        let idx = self.free.alloc(1).ok_or(kerr!(ENOMEM))?;
        if let Err(e) = self.map.raw_insert(idx, value) {
            // BitAlloc<N> exposes more bits than IndexMap has slots; release any index
            // that raw_insert rejects so a leaked bit can't accumulate across failures.
            self.free.free(idx, 1);
            return Err(e);
        }
        Ok(idx)
    }

    pub fn remove(&mut self, idx: &K) -> Option<V> {
        self.map.remove(idx).inspect(|_| {
            self.free.free(K::to_index(Some(idx)), 1);
        })
    }
}

impl<K: Copy + ToIndex, V, const N: usize> BitReclaimMap<K, V, N> {
    pub fn insert_with(&mut self, f: impl FnOnce(usize) -> Result<(K, V)>) -> Result<K> {
        let idx = self.free.alloc(1).ok_or(kerr!(ENOMEM))?;
        // The closure is user-supplied and may return an error; release the bit so a
        // sustained run of closure failures cannot exhaust the allocator.
        let (key, value) = match f(idx) {
            Ok(kv) => kv,
            Err(e) => {
                self.free.free(idx, 1);
                return Err(e);
            }
        };
        if let Err(e) = self.map.raw_insert(idx, value) {
            self.free.free(idx, 1);
            return Err(e);
        }
        Ok(key)
    }
}

impl<K: Copy + ToIndex, V, const N: usize> Index<K> for BitReclaimMap<K, V, N> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get::<K>(index).unwrap()
    }
}

impl<K: Copy + ToIndex, V, const N: usize> IndexMut<K> for BitReclaimMap<K, V, N> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut::<K>(index).unwrap()
    }
}

impl<K: ?Sized + ToIndex, V, const N: usize> Get<K> for BitReclaimMap<K, V, N> {
    type Output = V;

    fn get<Q: Borrow<K>>(&self, index: Q) -> Option<&Self::Output> {
        self.map.get(index)
    }
}

impl<K: ?Sized + ToIndex, V, const N: usize> GetMut<K> for BitReclaimMap<K, V, N> {
    fn get_mut<Q: Borrow<K>>(&mut self, index: Q) -> Option<&mut Self::Output> {
        self.map.get_mut(index)
    }

    fn get2_mut<Q: Borrow<K>>(
        &mut self,
        index1: Q,
        index2: Q,
    ) -> (Option<&mut Self::Output>, Option<&mut Self::Output>) {
        self.map.get2_mut(index1, index2)
    }

    fn get3_mut<Q: Borrow<K>>(
        &mut self,
        index1: Q,
        index2: Q,
        index3: Q,
    ) -> (
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
        Option<&mut Self::Output>,
    ) {
        self.map.get3_mut(index1, index2, index3)
    }
}

#[cfg(test)]
mod tests {
    use super::Vec;
    use crate::hal::mem::PhysAddr;
    use crate::mem::GLOBAL_ALLOCATOR;
    use core::ops::Range;
    use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    fn alloc_range(length: usize) -> Range<PhysAddr> {
        let alloc_range = std::alloc::Layout::from_size_align(length, align_of::<u128>()).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };
        PhysAddr::new(ptr as usize)..PhysAddr::new(ptr as usize + length)
    }

    fn setup_memory(mem_size: usize) {
        unsafe {
            GLOBAL_ALLOCATOR
                .lock()
                .add_range(&alloc_range(mem_size))
                .unwrap()
        };
    }

    #[derive(Debug)]
    struct Tracker<'a> {
        id: usize,
        drops: &'a AtomicUsize,
        drop_mask: &'a AtomicU64,
    }

    impl<'a> Tracker<'a> {
        fn new(id: usize, drops: &'a AtomicUsize, drop_mask: &'a AtomicU64) -> Self {
            Self {
                id,
                drops,
                drop_mask,
            }
        }
    }

    impl Drop for Tracker<'_> {
        fn drop(&mut self) {
            let bit = 1u64 << self.id;
            let old_mask = self.drop_mask.fetch_or(bit, Ordering::SeqCst);
            assert_eq!(old_mask & bit, 0, "value {} was dropped twice", self.id);
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct NonCopy {
        value: usize,
    }

    impl Clone for NonCopy {
        fn clone(&self) -> Self {
            Self { value: self.value }
        }
    }

    #[test]
    fn no_length_underflow() {
        let vec = Vec::<usize, 8>::new();
        assert!(vec.len() == 0);
        assert_eq!(vec.at(0), None);
    }

    #[test]
    fn reserve_works() {
        let mut vec = Vec::<usize, 8>::new();
        for i in 0..7usize {
            vec.push(i).unwrap();
        }
        assert_eq!(vec.len(), 7);

        let _ = vec.reserve(2);
    }

    #[test]
    fn drop_underflow() {
        let mut vec = Vec::<usize, 8>::new();
        for i in 0..7usize {
            vec.push(i).unwrap();
        }
        drop(vec);
    }

    #[test]
    fn push_and_index_non_copy_inline() {
        let mut vec = Vec::<NonCopy, 4>::new();

        for i in 0..4 {
            vec.push(NonCopy { value: i }).unwrap();
        }

        assert_eq!(vec.len(), 4);
        for i in 0..4 {
            assert_eq!(vec[i].value, i);
        }
    }

    #[test]
    fn push_grows_and_keeps_non_copy_values() {
        setup_memory(4096);
        let mut vec = Vec::<NonCopy, 2>::new();

        for i in 0..8 {
            vec.push(NonCopy { value: i }).unwrap();
        }

        assert_eq!(vec.len(), 8);
        assert!(vec.capacity() >= 8);
        for i in 0..8 {
            assert_eq!(vec.at(i).unwrap().value, i);
        }
    }

    #[test]
    fn reserve_moves_only_live_extra_values() {
        setup_memory(4096);
        let drops = AtomicUsize::new(0);
        let drop_mask = AtomicU64::new(0);
        let mut vec = Vec::<Tracker<'_>, 2>::new();

        for i in 0..4 {
            vec.push(Tracker::new(i, &drops, &drop_mask)).unwrap();
        }

        vec.reserve(10).unwrap();

        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert_eq!(vec.len(), 4);
        for i in 0..4 {
            assert_eq!(vec.at(i).unwrap().id, i);
        }

        drop(vec);
        assert_eq!(drops.load(Ordering::SeqCst), 4);
        assert_eq!(drop_mask.load(Ordering::SeqCst), 0b1111);
    }

    #[test]
    fn reserve_total_capacity_moves_non_copy_values() {
        setup_memory(4096);
        let mut vec = Vec::<NonCopy, 2>::new();

        for i in 0..5 {
            vec.push(NonCopy { value: i }).unwrap();
        }

        vec.reserve_total_capacity(16).unwrap();

        assert!(vec.capacity() >= 16);
        assert_eq!(vec.len(), 5);
        for i in 0..5 {
            assert_eq!(vec.at(i).unwrap().value, i);
        }
    }

    #[test]
    fn remove_from_inline_shifts_inline_values() {
        let mut vec = Vec::<NonCopy, 5>::new();
        for i in 0..5 {
            vec.push(NonCopy { value: i }).unwrap();
        }

        let removed = vec.remove(1).unwrap();

        assert_eq!(removed.value, 1);
        assert_eq!(vec.len(), 4);
        assert_eq!(vec.at(0).unwrap().value, 0);
        assert_eq!(vec.at(1).unwrap().value, 2);
        assert_eq!(vec.at(2).unwrap().value, 3);
        assert_eq!(vec.at(3).unwrap().value, 4);
    }

    #[test]
    fn remove_from_inline_pulls_first_extra_value_inline() {
        setup_memory(4096);
        let mut vec = Vec::<NonCopy, 3>::new();
        for i in 0..7 {
            vec.push(NonCopy { value: i }).unwrap();
        }

        let removed = vec.remove(1).unwrap();

        assert_eq!(removed.value, 1);
        assert_eq!(vec.len(), 6);
        for (idx, expected) in [0, 2, 3, 4, 5, 6].iter().copied().enumerate() {
            assert_eq!(vec.at(idx).unwrap().value, expected);
        }
    }

    #[test]
    fn remove_from_extra_shifts_extra_values() {
        setup_memory(4096);
        let mut vec = Vec::<NonCopy, 3>::new();
        for i in 0..8 {
            vec.push(NonCopy { value: i }).unwrap();
        }

        let removed = vec.remove(5).unwrap();

        assert_eq!(removed.value, 5);
        assert_eq!(vec.len(), 7);
        for (idx, expected) in [0, 1, 2, 3, 4, 6, 7].iter().copied().enumerate() {
            assert_eq!(vec.at(idx).unwrap().value, expected);
        }
    }

    #[test]
    fn remove_last_extra_does_not_shift_or_drop_extra_values() {
        setup_memory(4096);
        let drops = AtomicUsize::new(0);
        let drop_mask = AtomicU64::new(0);
        let mut vec = Vec::<Tracker<'_>, 2>::new();

        for i in 0..5 {
            vec.push(Tracker::new(i, &drops, &drop_mask)).unwrap();
        }

        let removed = vec.remove(4).unwrap();

        assert_eq!(removed.id, 4);
        assert_eq!(vec.len(), 4);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(removed);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        drop(vec);
        assert_eq!(drops.load(Ordering::SeqCst), 5);
        assert_eq!(drop_mask.load(Ordering::SeqCst), 0b1_1111);
    }

    #[test]
    fn pop_moves_value_out_without_copy() {
        let drops = AtomicUsize::new(0);
        let drop_mask = AtomicU64::new(0);
        let mut vec = Vec::<Tracker<'_>, 4>::new();

        for i in 0..3 {
            vec.push(Tracker::new(i, &drops, &drop_mask)).unwrap();
        }

        let popped = vec.pop().unwrap();

        assert_eq!(popped.id, 2);
        assert_eq!(vec.len(), 2);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(popped);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        drop(vec);
        assert_eq!(drops.load(Ordering::SeqCst), 3);
        assert_eq!(drop_mask.load(Ordering::SeqCst), 0b111);
    }

    #[test]
    fn clear_drops_each_live_value_once() {
        setup_memory(4096);
        let drops = AtomicUsize::new(0);
        let drop_mask = AtomicU64::new(0);
        let mut vec = Vec::<Tracker<'_>, 2>::new();

        for i in 0..6 {
            vec.push(Tracker::new(i, &drops, &drop_mask)).unwrap();
        }

        vec.clear();

        assert_eq!(vec.len(), 0);
        assert_eq!(drops.load(Ordering::SeqCst), 6);
        assert_eq!(drop_mask.load(Ordering::SeqCst), 0b11_1111);
        drop(vec);
        assert_eq!(drops.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn remove_then_drop_drops_every_value_once() {
        setup_memory(4096);
        let drops = AtomicUsize::new(0);
        let drop_mask = AtomicU64::new(0);
        let mut vec = Vec::<Tracker<'_>, 3>::new();

        for i in 0..7 {
            vec.push(Tracker::new(i, &drops, &drop_mask)).unwrap();
        }

        let removed_inline = vec.remove(1).unwrap();
        let removed_extra = vec.remove(4).unwrap();

        assert_eq!(removed_inline.id, 1);
        assert_eq!(removed_extra.id, 5);
        assert_eq!(drops.load(Ordering::SeqCst), 0);

        drop(removed_inline);
        drop(removed_extra);
        assert_eq!(drops.load(Ordering::SeqCst), 2);

        drop(vec);
        assert_eq!(drops.load(Ordering::SeqCst), 7);
        assert_eq!(drop_mask.load(Ordering::SeqCst), 0b111_1111);
    }

    #[test]
    fn clone_works_for_non_copy_values() {
        setup_memory(4096);
        let mut vec = Vec::<NonCopy, 2>::new();
        for i in 0..5 {
            vec.push(NonCopy { value: i }).unwrap();
        }

        let clone = vec.clone();

        assert_eq!(clone.len(), 5);
        for i in 0..5 {
            assert_eq!(clone.at(i).unwrap().value, i);
            assert_eq!(vec.at(i).unwrap().value, i);
        }
    }

    #[test]
    fn new_init_sets_length_and_initializes_values() {
        setup_memory(4096);
        let vec = Vec::<NonCopy, 2>::new_init(5, NonCopy { value: 42 }).unwrap();

        assert_eq!(vec.len(), 5);
        for i in 0..5 {
            assert_eq!(vec.at(i).unwrap().value, 42);
        }
    }

    #[test]
    fn new_init_oom_does_not_leak() {
        struct Counted<'a> {
            drops: &'a AtomicUsize,
            clones: &'a AtomicUsize,
        }
        impl Clone for Counted<'_> {
            fn clone(&self) -> Self {
                self.clones.fetch_add(1, Ordering::SeqCst);
                Counted {
                    drops: self.drops,
                    clones: self.clones,
                }
            }
        }
        impl Drop for Counted<'_> {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
            }
        }

        setup_memory(4096);
        let drops = AtomicUsize::new(0);
        let clones = AtomicUsize::new(0);
        let r = Vec::<Counted, 2>::new_init(
            1_000_000_000,
            Counted {
                drops: &drops,
                clones: &clones,
            },
        );
        assert!(r.is_err());
        let n_clones = clones.load(Ordering::SeqCst);
        let n_drops = drops.load(Ordering::SeqCst);
        assert_eq!(
            n_drops,
            n_clones + 1,
            "leaked clones (clones={}, drops={})",
            n_clones,
            n_drops,
        );
    }

    // -------- Vec::at2_mut / at3_mut bounds tests --------

    #[test]
    fn at2_mut_out_of_bounds_returns_none() {
        let mut vec = Vec::<usize, 4>::new();
        vec.push(7).unwrap();
        let (a, b) = vec.at2_mut(0, 2);
        assert!(a.is_some(), "index 0 is in-bounds");
        assert!(
            b.is_none(),
            "index 2 should be out-of-bounds (len=1) and return None, but got {:?}",
            b
        );
    }

    #[test]
    fn at3_mut_out_of_bounds_returns_none() {
        let mut vec = Vec::<usize, 4>::new();
        vec.push(7).unwrap();
        vec.push(8).unwrap();
        let (a, b, c) = vec.at3_mut(0, 1, 3);
        assert!(a.is_some());
        assert!(b.is_some());
        assert!(
            c.is_none(),
            "index 3 should be out-of-bounds (len=2) and return None, but got {:?}",
            c
        );
    }

    // -------- IndexMap::get2_mut / get3_mut bounds tests --------

    use super::IndexMap;
    use crate::types::traits::GetMut;

    #[test]
    fn indexmap_get2_mut_out_of_bounds_returns_none() {
        let mut m: IndexMap<usize, u32, 4> = IndexMap::new();
        m.raw_insert(0, 10).unwrap();
        let (a, b) = m.get2_mut(0usize, 10usize);
        assert!(a.is_some());
        assert!(b.is_none());
    }

    #[test]
    fn indexmap_get3_mut_out_of_bounds_returns_none() {
        let mut m: IndexMap<usize, u32, 4> = IndexMap::new();
        m.raw_insert(0, 10).unwrap();
        let (a, b, c) = m.get3_mut(0usize, 10usize, 11usize);
        assert!(a.is_some());
        assert!(b.is_none());
        assert!(c.is_none());
    }

    // -------- BitReclaimMap insert_with bit-leak test --------

    use super::BitReclaimMap;

    #[test]
    fn bitreclaim_insert_failure_does_not_leak() {
        let mut m: BitReclaimMap<usize, u32, 2> = BitReclaimMap::new();
        m.insert(10).unwrap();
        m.insert(20).unwrap();
        // Third insert allocates bit 2 then fails (raw_insert rejects idx >= N).
        assert!(m.insert(30).is_err());
        // Probe BitAlloc directly: bit 2 must be free again.
        let next_free = m.free.alloc(1);
        assert_eq!(
            next_free,
            Some(2),
            "expected bit 2 to be free after failed insert, but BitAlloc returned {:?}",
            next_free
        );
    }

    #[test]
    fn bitreclaim_insert_with_failed_closure_does_not_leak() {
        use crate::error::Result as KResult;
        let mut m: BitReclaimMap<usize, u32, 2> = BitReclaimMap::new();
        for _ in 0..10 {
            let r: KResult<usize> =
                m.insert_with(|_idx| -> KResult<(usize, u32)> { Err(kerr!(OutOfMemory)) });
            assert!(r.is_err());
        }
        let id0 = m.insert(10).unwrap();
        let id1 = m.insert(20).unwrap();
        assert!(id0 < 2);
        assert!(id1 < 2);
        assert_ne!(id0, id1);
    }
}
