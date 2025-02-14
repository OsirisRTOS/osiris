//! This module provides a queue implementation.

use super::array::IndexMap;
use crate::utils::KernelError;

/// A ring-buffer based queue, with N elements stored inline. TODO: Make this growable.
pub struct Queue<T: Clone, const N: usize> {
    data: IndexMap<T, N>,
    len: usize,
    front: usize,
}

impl<T: Clone + Copy, const N: usize> Queue<T, N> {
    /// Create a new empty queue.
    pub const fn new() -> Self {
        Self {
            data: IndexMap::new(),
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
        if self.len == N {
            return Err(KernelError::OutOfMemory);
        }

        let back = (self.front + self.len) % N;

        self.data.insert(back, value)?;
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

        let value = self.data.get(self.front).cloned();

        self.front = (self.front + 1) % N;
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
        self.data.insert((self.front + index) % N, value)
    }

    /// Returns the value at the front of the queue.
    pub fn front(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }

        self.data.get(self.front)
    }

    /// Returns the value at the back of the queue.
    pub fn back(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }

        let back = (self.front + self.len - 1) % N;
        self.data.get(back)
    }

    /// Returns the length of the queue.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
