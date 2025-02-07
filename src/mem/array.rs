use core::mem::MaybeUninit;

use super::alloc::AllocError;

pub struct IndexMap<T, const N: usize> {
    data: [Option<T>; N],
}

impl<T, const N: usize> IndexMap<T, N> {
    pub const fn new() -> Self {
        Self {
            data: [const { None }; N],
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < N {
            self.data[index].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < N {
            self.data[index].as_mut()
        } else {
            None
        }
    }

    pub fn insert(&mut self, index: usize, value: T) -> Result<(), AllocError> {
        if index < N {
            self.data[index] = Some(value);
            Ok(())
        } else {
            Err(AllocError::OutOfMemory)
        }
    }

    pub fn insert_next(&mut self, value: T) -> Result<usize, AllocError> {
        for (i, slot) in self.data.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(value);
                return Ok(i);
            }
        }

        Err(AllocError::OutOfMemory)
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < N {
            self.data[index].take()
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Option<T>> {
        self.data.iter()
    }

    pub fn iter_from_cycle(&self, index: usize) -> impl Iterator<Item = &Option<T>> {
        self.data.iter().cycle().skip(index + 1)
    }

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

use super::boxed::Box;

pub struct Vec<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
    extra: Box<[MaybeUninit<T>]>
}

impl<T: Clone + Copy, const N: usize> Vec<T, N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [const { MaybeUninit::uninit() }; N],
            extra: Box::new_slice_empty(),
        }
    }

    pub fn reserve(&mut self, additional: usize) -> Result<(), AllocError> {
        let len_extra = self.extra.len();

        if self.len + additional <= N + len_extra {
            return Ok(());
        }

        let grow = additional - N + len_extra;
        let mut new_extra = Box::new_slice_uninit(grow)?;

        BUG_ON!(new_extra.len() != grow);

        new_extra[..len_extra].copy_from_slice(&self.extra);

        self.extra = new_extra;
        Ok(())
    }

    pub fn new_init(length: usize, value: T) -> Result<Self, AllocError> {
        let mut vec = Self::new();

        if length <= N {
            for i in 0..length {
                vec.data[i].write(value);
            }
        } else {
            vec.data.fill(MaybeUninit::new(value));

            if length - N > 0 {
                let mut extra = Box::new_slice_uninit(length - N)?;

                for i in N..length {
                    extra[i-N].write(value);
                }

                vec.extra = extra;
            }
        }

        Ok(vec)
    }

    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        if self.len < N {
            self.data[self.len].write(value);
            self.len += 1;
            Ok(())
        } else {
            let len_extra = self.extra.len();
            
            if self.len < N + len_extra {
                self.extra[self.len - N].write(value);
                self.len += 1;
                Ok(())
            } else {
                let grow = (len_extra + 1) * 2;
                let mut new_extra = Box::new_slice_uninit(grow)?;

                BUG_ON!(new_extra.len() != grow);

                new_extra[..len_extra].copy_from_slice(&self.extra);

                self.extra = new_extra;
                self.extra[len_extra].write(value);
                self.len += 1;
                Ok(())
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.remove(self.len - 1)
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }

        let value = self.at(index).cloned();

        if index < N {
            let end = core::cmp::min(self.len, N);

            // Safety: index is less than N and min too.
            self.data.copy_within(index+1..end, index);

            if let Some(value) = self.at(N) {
                self.data[end-1].write(*value);
            }

            if self.len() > N {
                self.extra.copy_within(1..self.len - N, 0);
            }
        } else {
            let index = index - N;
            let end = self.len - N;

            // Safety: index is less than N and min too.
            self.extra.copy_within(index+1..end, index);
        }

        self.len -= 1;
        value
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn at(&self, index: usize) -> Option<&T> {
        if index > self.len - 1 {
            return None;
        }

        if index < N {
            unsafe { Some(self.data[index].assume_init_ref()) }
        } else {
            let index = index - N;
            unsafe { Some(self.extra[index].assume_init_ref()) }
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a >= self.len || b >= self.len {
            return;
        }

        if a < N && b < N {
            self.data.swap(a, b);
        } else if a >= N && b >= N {
            self.extra.swap(a - N, b - N);
        } else if a >= N {
            core::mem::swap(&mut self.extra[a-N], &mut self.data[b]);
        } else {
            core::mem::swap(&mut self.data[a], &mut self.extra[b-N]);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T, const N: usize> Drop for Vec<T, N> {
    fn drop(&mut self) {
        let min = core::cmp::min(self.len, N);

        for elem in &mut self.data[0..min] {
            unsafe { elem.assume_init_drop(); }
        }

        for elem in &mut (*self.extra)[0..self.len - N] {
            unsafe { elem.assume_init_drop(); }
        }
    }
}