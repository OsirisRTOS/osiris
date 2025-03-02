//! This module provides a queue implementation.

use super::array::Vec;
use super::boxed::Box;
use crate::utils::KernelError;

/// A ring-buffer based queue, with N elements stored inline. TODO: Make this growable.
pub struct Queue<T: Clone, const N: usize> {
    data: Vec<T, N>,
    len: usize,
    front: usize,
}

impl<T: Clone + Copy, const N: usize> Queue<T, N> {
    /// Create a new empty queue.
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            len: 0,
            front: 0,
        }
    }

    /// Push a value onto the back of the queue.
    ///
    /// `value` - The value to push onto the back of the queue.
    ///
    /// Returns `Ok(())` if the value was pushed onto the back of the queue, or an error if the queue is full.
    pub fn push_back(&mut self, value: T) -> Result<(), KernelError> {
        if self.len == self.data.len() {
            return Err(KernelError::OutOfMemory);
        }

        let back = (self.front + self.len) % self.data.len();
        let insertion_point = self.data.at_mut(back).ok_or(KernelError::InvalidAddress)?;
        *insertion_point = value;

        self.len += 1;
        Ok(())
    }

    /// Pop a value from the front of the queue.
    ///
    /// Returns the value at the front of the queue, or `None` if the queue is empty.
    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let value = self.data.at(self.front).cloned();

        self.front = (self.front + 1) % self.data.len();
        self.len -= 1;
        value
    }

    /// Insert a value at the given index in the queue.
    ///
    /// `index` - The index to insert the value at.
    /// `value` - The value to insert.
    ///
    /// Returns `Ok(())` if the value was inserted at the given index, or an error if the index is out of bounds.
    pub fn insert(&mut self, index: usize, value: T) -> Result<(), KernelError> {
        self.data
            .at_mut((self.front + index) % N)
            .map(|insertion_point| *insertion_point = value)
            .ok_or(KernelError::InvalidAddress)
    }

    /// Returns the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }

        self.data.at(self.front)
    }

    /// Returns the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }

        let back = (self.front + self.len - 1) % self.data.len();
        self.data.at(back)
    }

    /// Returns the length of the queue.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn grow_capacity(&mut self, new_size: usize) -> Result<(), KernelError> {
        if new_size < self.data.len() {
            return Ok(());
        }
        // if the queue wraps
        if self.front + self.len >= self.data.len() {
            // copy the queue to the front to make further logic straighforward
            // When the queue wraps around the end, the wrapping would not happen anymore with the new size

            // we could do some complicated in-place swapping here instead of using a potentially expensive temporary storage
            let mut swap_helper = Box::new_slice_uninit(self.data.len() - self.front)?;
            for i in 0..swap_helper.len() {
                // Returning an error here should never happen if the queue is in a consistant state prior. If not no guarantees about contents are made.
                swap_helper[i].write(
                    self.data
                        .at(self.front + i)
                        .copied()
                        .ok_or(KernelError::InvalidAddress)?,
                );
            }
            let end = (self.front + self.len) % self.data.len();
            for i in 0..end {
                BUG_ON!(i + swap_helper.len() >= self.data.len());
                self.data.swap(i, i + swap_helper.len());
            }
            // now copy the data back from the temp helper
            for i in 0..swap_helper.len() {
                // Safety: values copied into our helper are part of the active queue, must therefore be initedF
                self.data
                    .at_mut(i)
                    .map(|el| *el = unsafe { swap_helper[i].assume_init() });
            }
            self.front = 0;
        }
        self.data.reserve(new_size - self.data.len())
    }
}
