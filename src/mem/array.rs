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

impl<T: Clone, const N: usize> Vec<T, N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [const { MaybeUninit::uninit() }; N],
            extra: Box::new_slice_empty(),
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        if self.len < N {
            self.data[self.len].write(value);
            self.len += 1;
            Ok(())
        } else {
            let extra = self.extra.len();
            
            if self.len < N + extra {
                self.extra[self.len - N].write(value);
                self.len += 1;
                Ok(())
            } else {
                let grow = (extra + 1) * 2;
                let mut new_extra = Box::new_slice_uninit(grow)?;

                BUG_ON!(new_extra.len() != grow);

                // Only the first len - N elements are initialized.
                for (i, elem) in self.extra.iter_mut().enumerate() {
                    if i == self.len - N {
                        break;
                    }
                    let new_elem = unsafe { elem.assume_init_ref().clone() };
                    BUG_ON!(i >= new_extra.len());
                    new_extra[i].write(new_elem);
                    //unsafe { elem.assume_init_drop() };
                }

                self.extra = new_extra;
                self.extra[extra].write(value);
                self.len += 1;
                Ok(())
            }
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len {
            return None;
        }

        if index < N {
            // Safety: `index` is less then self.len. That means the element at `index` is initialized.
            let value = unsafe { self.data[index].assume_init_read() };
            let min = core::cmp::min(self.len, N);

            for i in index..min - 1 {
                self.data[i].write(unsafe { self.data[i + 1].assume_init_read() });
            }

            if self.len > N {
                for i in 0..self.len - N - 2 {
                    let value = unsafe { self.extra[i+1].assume_init_read() };
                    self.extra[i].write(value);
                }
            }

            self.len -= 1;
            Some(value)
        } else {
            let index = index - N;
            // Safety: `index` is less then self.len. That means the element at `index` is initialized.
            let value = unsafe { self.extra[index].assume_init_read() };
            for i in index..self.len - N - 2 {
                let value = unsafe { self.extra[i+1].assume_init_read() };
                self.extra[i].write(value);
            }

            self.len -= 1;
            Some(value)
        }
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
        if a < N && b < N {
            self.data.swap(a, b);
        } else if a >= N && b >= N {
            self.extra.swap(a - N, b - N);
        } else if a >= N {
            let helper = unsafe { self.extra[a-N].assume_init_ref().clone()};
            self.extra[a-N].write(unsafe { self.data[b].assume_init_ref().clone() });
            self.data[b].write(helper);
        } else {
            let helper = unsafe { self.data[a].assume_init_ref().clone()};
            self.data[a].write(unsafe { self.extra[b-N].assume_init_ref().clone() });
            self.extra[b-N].write(helper);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T, const N: usize> Drop for Vec<T, N> {
    fn drop(&mut self) {
        let min = core::cmp::min(self.len, N);
        for elem in &mut self.data[0..min-1] {
            unsafe { elem.assume_init_drop(); }
        }

        for elem in &mut (*self.extra)[0..self.len - N-1] {
            unsafe { elem.assume_init_drop(); }
        }
    }
}