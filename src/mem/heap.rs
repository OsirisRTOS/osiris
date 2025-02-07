use super::{alloc::AllocError, array::Vec};

pub struct PriorityQueue<T> {
    vec: Vec<T, 32>,
}

impl<T: Clone + Copy + Ord> PriorityQueue<T> {
    pub const fn new() -> Self {
        Self {
            vec: Vec::new(),
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        self.vec.push(value)?;
        self.sift_up(self.len() - 1);
        Ok(())
    }

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

    fn sift_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.vec.at(parent) <= self.vec.at(index) {
                break;
            }
            self.vec.swap(parent, index);
            index = parent;
        }
    }

    fn sift_down(&mut self, mut index: usize) {
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn peek(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        self.vec.at(0)
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }
}

