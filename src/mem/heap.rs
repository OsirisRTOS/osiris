//! This module provides a binary heap implementation.

use super::array::Vec;
use crate::utils::KernelError;

/// An array-based binary heap, with N elements stored inline.
pub struct BinaryHeap<T, const N: usize> {
    vec: Vec<T, N>,
}

impl<T: Clone + Copy + Ord, const N: usize> BinaryHeap<T, N> {

    /// Create a new empty binary heap.
    pub const fn new() -> Self {
        Self { vec: Vec::new() }
    }

    /// Push a value onto the binary heap.
    /// 
    /// `value` - The value to push onto the binary heap.
    /// 
    /// Returns `Ok(())` if the value was pushed onto the binary heap, or an error if the heap cannot be extended (e.g. OOM).
    pub fn push(&mut self, value: T) -> Result<(), KernelError> {
        self.vec.push(value)?;
        self.sift_up(self.len() - 1);
        Ok(())
    }

    /// Pop the smallest value from the binary heap.
    /// 
    /// Returns the smallest value in the binary heap, or `None` if the heap is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.peek().cloned();
        self.vec.swap(0, self.len() - 1);
        self.vec.pop();
        self.sift_down(0);
        value
    }

    /// Sift the value at the given index up the binary heap.
    /// 
    /// `index` - The index of the value to sift up.
    fn sift_up(&mut self, mut index: usize) {
        // We move up the heap until we reach the root or the parent is smaller than the current value.
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.vec.at(parent) <= self.vec.at(index) {
                break;
            }
            self.vec.swap(parent, index);
            index = parent;
        }
    }

    /// Sift the value at the given index down the binary heap.
    /// 
    /// `index` - The index of the value to sift down.
    fn sift_down(&mut self, mut index: usize) {
        // We move down the heap until we reach a leaf or the value is smaller than both children.
        while index < self.len() {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            let mut smallest = index;

            if left < self.len() && self.vec.at(left) < self.vec.at(smallest) {
                smallest = left;
            }

            if right < self.len() && self.vec.at(right) < self.vec.at(smallest) {
                smallest = right;
            }

            if smallest == index {
                break;
            }

            self.vec.swap(smallest, index);
            index = smallest;
        }
    }


    /// Check if the binary heap is empty.
    /// 
    /// Returns `true` if the binary heap is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Peek at the smallest value in the binary heap.
    /// 
    /// Returns the smallest value in the binary heap, or `None` if the heap is empty.
    pub fn peek(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        self.vec.at(0)
    }

    /// Get the number of elements in the binary heap.
    pub fn len(&self) -> usize {
        self.vec.len()
    }
}
