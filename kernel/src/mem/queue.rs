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
        if self.len == self.data.capacity() {
            return Err(KernelError::OutOfMemory);
        }
        self.len += 1;
        if self.data.len() != self.data.capacity() {
            self.data.push(value)?;
        } else {
            self.insert(self.len - 1, value)?;
        }

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

        self.front = (self.front + 1) % self.data.capacity();
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
        if index >= self.len() {
            return Err(KernelError::InvalidAddress);
        }
        let real_idx = (self.front + index) % self.data.capacity();
        self.data
            .at_mut(real_idx)
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
        let back = (self.front + self.len - 1) % self.data.capacity();
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

    /// Increases total space in the queue preserving queue-integrity, potentially reallocating and copying existing contents
    ///
    /// `new_size` - The total amount of space in the queue afterwards
    ///
    /// Returns `Ok(())` if the queue was successfully enlargened or the requested size was smaller than the current capacity.
    /// Returns An error if the queue could not be grown
    pub fn grow_capacity(&mut self, new_size: usize) -> Result<(), KernelError> {
        if new_size <= self.data.capacity() {
            return Ok(());
        }
        // if the queue wraps
        if self.front + self.len >= self.data.capacity() {
            // copy the queue to the front to make further logic straighforward
            // When the queue wraps around the end, the wrapping would not happen anymore with the new size

            // we could do some complicated in-place swapping here instead of using a potentially expensive temporary storage
            let non_wrapping_queue_start_len = self.data.capacity() - self.front;
            let mut swap_helper = Box::new_slice_uninit(non_wrapping_queue_start_len)?;
            BUG_ON!(swap_helper.len() != non_wrapping_queue_start_len);

            // we take the start of the queue (which is located at the end of the curr memory region) and copy it to temp storage
            for i in 0..swap_helper.len() {
                // Returning an error here should never happen if the queue is in a consistant state prior. If not no guarantees about contents are made.
                swap_helper[i].write(
                    self.data
                        .at(self.front + i)
                        .copied()
                        .ok_or(KernelError::InvalidAddress)?,
                );
            }
            // One past the logically last element of the queue
            let end = (self.front + self.len) % self.data.capacity();
            // now move the logical end of the queue further back to make space for the logical start
            for i in 0..end {
                BUG_ON!(i + non_wrapping_queue_start_len >= self.data.capacity());
                self.data.swap(i, i + non_wrapping_queue_start_len);
            }
            // now copy the data back from the temp helper
            for i in 0..non_wrapping_queue_start_len {
                // Safety: values copied into our helper are part of the active queue, must therefore be inited
                self.data
                    .at_mut(i)
                    .map(|el| *el = unsafe { swap_helper[i].assume_init() });
            }
            self.front = 0;
        }
        self.data.reserve_total_capacity(new_size)
    }
}

// TESTING ------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::GLOBAL_ALLOCATOR;
    use core::ops::Range;

    fn alloc_range(length: usize) -> Range<usize> {
        let alloc_range = std::alloc::Layout::from_size_align(length, align_of::<u128>()).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };
        ptr as usize..ptr as usize + length
    }

    fn setup_memory(mem_size: usize) {
        unsafe {
            GLOBAL_ALLOCATOR
                .lock()
                .add_range(alloc_range(mem_size))
                .unwrap()
        };
    }

    #[test]
    fn growing_retains_queue_state_without_wrapping() {
        setup_memory(1000);
        let mut queue = Queue::<usize, 10>::new();
        for i in 0..10 {
            assert_eq!(queue.push_back(i), Ok(()));
        }

        assert_eq!(queue.grow_capacity(20), Ok(()));
        for i in 0..10 {
            assert_eq!(queue.pop_front(), Some(i));
        }
    }

    #[test]
    fn growing_retains_queue_state_with_wrapping() {
        setup_memory(1000);
        let mut queue = Queue::<usize, 10>::new();
        for i in 0..10 {
            assert_eq!(queue.push_back(i), Ok(()));
        }
        // sanity check that queue really is full
        assert_eq!(queue.push_back(1), Err(KernelError::OutOfMemory));
        assert_eq!(queue.len(), 10);

        // pop and subsequently push more elements to make queue wrap
        for i in 0..5 {
            assert_eq!(queue.pop_front(), Some(i));
        }

        assert_eq!(*queue.front().unwrap(), 5);
        assert_eq!(*queue.back().unwrap(), 9);
        assert_eq!(queue.len(), 5);

        for i in 10..15 {
            assert_eq!(queue.push_back(i), Ok(()));
        }

        assert_eq!(queue.len(), 10);
        assert_eq!(*queue.front().unwrap(), 5);
        assert_eq!(*queue.back().unwrap(), 14);
        assert_eq!(queue.grow_capacity(20), Ok(()));
        for i in 5..15 {
            assert_eq!(queue.pop_front(), Some(i));
        }
    }

    #[test]
    fn growing_to_smaller_size_has_no_effect() {
        setup_memory(1000);
        let mut queue = Queue::<usize, 10>::new();
        for i in 0..10 {
            assert_eq!(queue.push_back(i), Ok(()));
        }
        assert_eq!(queue.len(), 10);
        queue.grow_capacity(1).unwrap();
        assert_eq!(queue.len(), 10);
        assert_eq!(queue.push_back(1), Err(KernelError::OutOfMemory));
    }

    #[test]
    fn growing_multiple_times_consecutively_retains_state() {
        setup_memory(10000);
        let mut queue = Queue::<usize, 10>::new();
        for i in 0..10 {
            assert_eq!(queue.push_back(i), Ok(()));
        }
        assert_eq!(queue.len(), 10);

        // pop and subsequently push more elements to make queue wrap
        for i in 0..5 {
            assert_eq!(queue.pop_front(), Some(i));
        }
        assert_eq!(queue.len(), 5);

        for i in 10..15 {
            assert_eq!(queue.push_back(i), Ok(()));
        }

        assert_eq!(queue.len(), 10);
        assert_eq!(*queue.front().unwrap(), 5);
        assert_eq!(*queue.back().unwrap(), 14);
        assert_eq!(queue.grow_capacity(20), Ok(()));
        assert_eq!(queue.grow_capacity(40), Ok(()));
        assert_eq!(queue.grow_capacity(100), Ok(()));
        assert_eq!(queue.grow_capacity(200), Ok(()));
        for i in 5..15 {
            assert_eq!(queue.pop_front(), Some(i));
        }
    }
}
// END TESTING

// VERIFICATION -------------------------------------------------------------------------------------------------------
#[cfg(kani)]
mod verification {
    use super::*;
    use core::cmp::max;
    use std::cmp::min;
    use std::vec::Vec;

    #[test]
    fn kani_concrete_playback_growing_retains_queue_state_with_wrapping_7154119071478699851() {
        let concrete_vals: Vec<Vec<u8>> = vec![
            // 99968ul
            vec![128, 134, 1, 0, 0, 0, 0, 0],
        ];
        kani::concrete_playback_run(concrete_vals, growing_retains_queue_state_with_wrapping);
    }

    //#[kani::proof]
    //#[kani::unwind(15)]
    fn growing_retains_queue_state_with_wrapping() {
        let mut queue = Queue::<usize, 10>::new();
        for i in 0..10 {
            assert_eq!(queue.push_back(i), Ok(()));
        }
        // sanity check that queue really is full
        assert_eq!(queue.push_back(1), Err(KernelError::OutOfMemory));
        assert_eq!(queue.len(), 10);

        // pop and subsequently push more elements to make queue wrap
        for i in 0..5 {
            assert_eq!(queue.pop_front(), Some(i));
        }

        assert_eq!(*queue.front().unwrap(), 5);
        assert_eq!(*queue.back().unwrap(), 9);
        assert_eq!(queue.len(), 5);

        for i in 10..15 {
            assert_eq!(queue.push_back(i), Ok(()));
        }

        assert_eq!(queue.len(), 10);
        assert_eq!(*queue.front().unwrap(), 5);
        assert_eq!(*queue.back().unwrap(), 14);
        let new_cap = kani::any();
        let res = queue.grow_capacity(new_cap);

        if res == Ok(()) && new_cap > 10 {
            for i in 0..(new_cap - 10) {
                // add some more elements for good measure
                assert_eq!(queue.push_back(i), Ok(()));
            }
        }

        for i in 5..15 {
            assert_eq!(queue.pop_front(), Some(i));
        }

        while !queue.is_empty() {
            let _ = queue.pop_front();
        }

        // now add and remove elements again to show queue still works as expected
        if res == Ok(()) && new_cap > 10 {
            for i in 0..new_cap {
                assert_eq!(queue.push_back(i), Ok(()));
            }

            for i in 0..new_cap {
                assert_eq!(queue.pop_front(), Some(i));
            }
        }
    }
}
// END VERIFICATION
