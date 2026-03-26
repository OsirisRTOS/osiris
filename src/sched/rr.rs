use crate::{
    sched::{
        thread::{self},
    },
    types::{
        list::List,
    },
};

pub struct Scheduler<const N: usize> {
    queue: List<thread::RRList, thread::UId>,

    current: Option<thread::UId>,
    current_left: u64,
    quantum: u64,
}

impl<const N: usize> Scheduler<N> {
    pub const fn new() -> Self {
        // TODO: Make quantum configurable.
        Self { queue: List::new(), current: None, current_left: 0, quantum: 1000 }
    }

    pub fn enqueue(&mut self, uid: thread::UId, storage: &mut super::ThreadMap<N>) -> Result<(), crate::utils::KernelError> {
        self.queue.push_back(uid, storage).map_err(|_| crate::utils::KernelError::InvalidArgument)
    }

    pub fn put(&mut self, uid: thread::UId, dt: u64) {
        if let Some(current) = self.current {
            if current == uid {
                self.current_left = self.current_left.saturating_sub(dt);
            }
        }
    }

    pub fn pick(&mut self, storage: &mut super::ThreadMap<N>) -> Option<(thread::UId, u64)> {
        if self.current_left == 0 {
            if let Some(current) = self.current {
                self.queue.push_back(current, storage);
            }

            self.current = self.queue.pop_front(storage).ok().flatten();
            self.current_left = self.quantum;
        }

        self.current.map(|id| (id, self.current_left))
    }
}
