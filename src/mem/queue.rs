use super::{alloc::AllocError, array::IndexMap};

pub struct Queue<T: Clone, const N: usize> {
    data: IndexMap<T, N>,
    len: usize,
    front: usize,
}

impl<'a, T: Clone, const N: usize> Queue<T, N> {
    pub const fn new() -> Self {
        Self {
            data: IndexMap::new(),
            len: 0,
            front: 0,
        }
    }

    pub fn push_back(&mut self, value: T) -> Result<(), AllocError> {
        if self.len == N {
            return Err(AllocError::OutOfMemory);
        }

        let back = (self.front + self.len) % N;
        self.data.insert(back, value)?;
        self.len += 1;
        Ok(())
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let value = self.data.get(self.front).cloned();

        self.front = (self.front + 1) % N;
        self.len -= 1;
        value
    }

    pub fn insert(&mut self, index: usize, value: T) -> Result<(), AllocError> {
        self.data.insert((self.front + index) % N, value)
    }

    pub fn front(&self) -> Option<&T> {
        self.data.get(self.front)
    }

    pub fn back(&self) -> Option<&T> {
        if self.len == 0 {
            return None;
        }

        let back = (self.front + self.len - 1) % N;
        self.data.get(back)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}