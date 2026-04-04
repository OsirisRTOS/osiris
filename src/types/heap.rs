//! This module provides a binary heap implementation.

use crate::error::Result;

use super::array::Vec;

/// An array-based binary heap, with N elements stored inline.
#[proc_macros::fmt]
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
    pub fn push(&mut self, value: T) -> Result<()> {
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


#[cfg(kani)]
mod verification {
    use super::BinaryHeap;

    /// Verify that pushing a single element and popping it returns the same element.
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_push_pop_roundtrip() {
        let mut heap = BinaryHeap::<u32, 4>::new();
        let v: u32 = kani::any();
        heap.push(v).unwrap();
        let popped = heap.pop();
        assert_eq!(popped, Some(v));
        assert!(heap.is_empty());
    }

    /// Verify that pushing two elements and popping gives the smaller one first (min-heap).
    #[kani::proof]
    #[kani::unwind(5)]
    fn verify_min_heap_two_elements() {
        let mut heap = BinaryHeap::<u32, 4>::new();
        let a: u32 = kani::any();
        let b: u32 = kani::any();
        heap.push(a).unwrap();
        heap.push(b).unwrap();
        let first = heap.pop().unwrap();
        let second = heap.pop().unwrap();
        // Min-heap: first <= second, and {first, second} == {a, b}
        assert!(first <= second);
        assert!((first == a && second == b) || (first == b && second == a));
    }

    /// Verify that pushing three elements pops them in non-decreasing order.
    #[kani::proof]
    #[kani::unwind(6)]
    fn verify_min_heap_three_elements_sorted() {
        let mut heap = BinaryHeap::<u32, 4>::new();
        let a: u32 = kani::any();
        let b: u32 = kani::any();
        let c: u32 = kani::any();
        heap.push(a).unwrap();
        heap.push(b).unwrap();
        heap.push(c).unwrap();
        let x = heap.pop().unwrap();
        let y = heap.pop().unwrap();
        let z = heap.pop().unwrap();
        // Must come out in non-decreasing order.
        assert!(x <= y);
        assert!(y <= z);
    }

    /// Verify that peek() always returns the minimum element after arbitrary pushes.
    #[kani::proof]
    #[kani::unwind(6)]
    fn verify_peek_is_minimum() {
        let mut heap = BinaryHeap::<u32, 4>::new();
        let a: u32 = kani::any();
        let b: u32 = kani::any();
        let c: u32 = kani::any();
        heap.push(a).unwrap();
        heap.push(b).unwrap();
        heap.push(c).unwrap();
        let peeked = *heap.peek().unwrap();
        // peeked must be <= all elements
        assert!(peeked <= a);
        assert!(peeked <= b);
        assert!(peeked <= c);
    }
}


#[cfg(test)]
mod tests {
    use super::BinaryHeap;

    #[test]
    fn test_heap_sorted_order() {
        let mut heap = BinaryHeap::<u32, 8>::new();
        for &v in &[5u32, 2, 8, 1, 9, 3] {
            heap.push(v).unwrap();
        }
        let mut prev = 0u32;
        while let Some(v) = heap.pop() {
            assert!(v >= prev, "heap pop out of order: {} after {}", v, prev);
            prev = v;
        }
    }

    #[test]
    fn test_heap_single_element() {
        let mut heap = BinaryHeap::<u32, 4>::new();
        heap.push(42).unwrap();
        assert_eq!(heap.peek(), Some(&42));
        assert_eq!(heap.pop(), Some(42));
        assert!(heap.is_empty());
    }

    #[test]
    fn test_heap_empty_peek_pop() {
        let mut heap = BinaryHeap::<u32, 4>::new();
        assert!(heap.peek().is_none());
        assert!(heap.pop().is_none());
   }

    #[test]
    fn test_heap_duplicate_values() {
        let mut heap = BinaryHeap::<u32, 4>::new();
        heap.push(3).unwrap();
        heap.push(3).unwrap();
        heap.push(1).unwrap();
        assert_eq!(heap.pop(), Some(1));
        assert_eq!(heap.pop(), Some(3));
        assert_eq!(heap.pop(), Some(3));
    }
}

