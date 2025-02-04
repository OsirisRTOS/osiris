use super::{alloc::AllocError, array::Vec};

pub struct PriorityQueue<'a, T> {
    heap: Vec<'a, T, 32>,
    size: usize,
}

impl<T: Clone + Ord> PriorityQueue<'_, T> {
    pub const fn new() -> Self {
        Self {
            heap: Vec::new(),
            size: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        self.heap.push(value)?;
        self.size += 1;
        self.sift_up(self.size - 1);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }

        let value = self.peek().cloned();
        self.size -= 1;
        self.sift_down(0);
        value
    }

    fn sift_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.heap.at(parent) <= self.heap.at(index) {
                break;
            }
            self.heap.swap(parent, index);
            index = parent;
        }
    }

    fn sift_down(&mut self, mut index: usize) {
        while index < self.size {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            let mut smallest = index;

            if left < self.size && self.heap.at(left) < self.heap.at(smallest) {
                smallest = left;
            }

            if right < self.size && self.heap.at(right) < self.heap.at(smallest) {
                smallest = right;
            }

            if smallest == index {
                break;
            }

            self.heap.swap(smallest, index);
            index = smallest;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn peek(&self) -> Option<&T> {
        if self.size == 0 {
            return None;
        }
        self.heap.at(0)
    }

    pub fn size(&self) -> usize {
        self.size
    }
}