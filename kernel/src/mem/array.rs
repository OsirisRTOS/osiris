//! This module implements static and dynamic arrays for in-kernel use.

use super::boxed::Box;
use crate::utils::KernelError;
use core::mem::MaybeUninit;

/// This is a fixed-size map that can store up to N consecutive elements.
pub struct IndexMap<T, const N: usize> {
    data: [Option<T>; N],
}

impl<T, const N: usize> IndexMap<T, N> {
    /// Create a new IndexMap.
    ///
    /// Returns a new IndexMap.
    pub const fn new() -> Self {
        Self {
            data: [const { None }; N],
        }
    }

    /// Get the element at the given index.
    ///
    /// `index` - The index to get the element from.
    ///
    /// Returns `Some(&T)` if the index is in-bounds, otherwise `None`.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < N {
            self.data[index].as_ref()
        } else {
            None
        }
    }

    /// Get a mutable reference to the element at the given index.
    ///
    /// `index` - The index to get the element from.
    ///
    /// Returns `Some(&mut T)` if the index is in-bounds, otherwise `None`.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < N {
            self.data[index].as_mut()
        } else {
            None
        }
    }

    /// Insert a value at the given index.
    ///
    /// `index` - The index to insert the value at.
    /// `value` - The value to insert.
    ///
    /// Returns `Ok(())` if the index was in-bounds, otherwise `Err(KernelError::OutOfMemory)`.
    pub fn insert(&mut self, index: usize, value: T) -> Result<(), KernelError> {
        if index < N {
            self.data[index] = Some(value);
            Ok(())
        } else {
            Err(KernelError::OutOfMemory)
        }
    }

    /// Insert a value at the next available index.
    ///
    /// `value` - The value to insert.
    ///
    /// Returns `Ok(index)` if the value was inserted, otherwise `Err(KernelError::OutOfMemory)`.
    pub fn insert_next(&mut self, value: T) -> Result<usize, KernelError> {
        for (i, slot) in self.data.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(value);
                return Ok(i);
            }
        }

        Err(KernelError::OutOfMemory)
    }

    /// Remove the value at the given index.
    ///
    /// `index` - The index to remove the value from.
    ///
    /// Returns the value if it was removed, otherwise `None`.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < N {
            self.data[index].take()
        } else {
            None
        }
    }

    /// Get an iterator over the elements in the map.
    ///
    /// Returns an iterator over the elements in the map.
    pub fn iter(&self) -> impl Iterator<Item = &Option<T>> {
        self.data.iter()
    }

    /// Get an cycling iterator over the elements in the map, starting from the given index.
    ///
    /// `index` - The index to start the iterator from.
    ///
    /// Returns an iterator over the elements in the map.
    pub fn iter_from_cycle(&self, index: usize) -> impl Iterator<Item = &Option<T>> {
        self.data.iter().cycle().skip(index + 1)
    }

    /// Get the next index that contains a value (this will cycle).
    ///
    /// `index` - The index to start the search from.
    ///
    /// Returns the next index (potentially < index) that contains a value, otherwise `None`.
    pub fn next(&self, index: Option<usize>) -> Option<usize> {
        let index = index.unwrap_or(0);

        for (i, elem) in self.iter_from_cycle(index).enumerate() {
            if elem.is_some() {
                return Some((index + i + 1) % N);
            }
        }

        None
    }
}

/// This is a vector that can store up to N elements inline and will allocate on the heap if more are needed.
pub struct Vec<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
    extra: Box<[MaybeUninit<T>]>,
}

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
    pub fn reserve(&mut self, additional: usize) -> Result<(), KernelError> {
        let len_extra = self.extra.len();

        // Check if we have enough space in the inline storage.
        if self.len + additional <= N + len_extra {
            return Ok(());
        }

        // If we don't have enough space, we need to grow the extra storage.
        let grow = additional - N + len_extra;
        let mut new_extra = Box::new_slice_uninit(grow)?;

        // Check that the new extra storage has the requested length.
        BUG_ON!(new_extra.len() != grow);

        // Copy the old extra storage into the new one.
        new_extra[..len_extra].copy_from_slice(&self.extra);

        // Replace the old extra storage with the new one. The old one will be dropped.
        self.extra = new_extra;
        Ok(())
    }

    pub fn reserve_total_capacity(&mut self, total_capacity: usize) -> Result<(), KernelError> {
        // Check if we already have enough space
        if self.capacity() >= total_capacity {
            return Ok(());
        }

        // If we don't have enough space, we need to grow the extra storage.
        let new_out_of_line_cap = total_capacity - N;
        let mut new_extra = Box::new_slice_uninit(new_out_of_line_cap)?;

        // Check that the new extra storage has the requested length.
        BUG_ON!(new_extra.len() != new_out_of_line_cap);

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
    pub fn new_init(length: usize, value: T) -> Result<Self, KernelError> {
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
    pub fn push(&mut self, value: T) -> Result<(), KernelError> {
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

                BUG_ON!(new_extra.len() != grow);

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
