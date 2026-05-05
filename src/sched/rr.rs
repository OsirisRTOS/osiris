use crate::{
    error::Result,
    sched::thread::{self},
    types::list::List,
};

pub struct Scheduler<const N: usize> {
    queue: List<thread::RRList, thread::UId>,

    current: Option<thread::UId>,
    current_left: u32,
    quantum: u32,
}

impl<const N: usize> Scheduler<N> {
    pub const fn new() -> Self {
        // TODO: Make quantum configurable.
        Self {
            queue: List::new(),
            current: None,
            current_left: 0,
            quantum: 1000,
        }
    }

    pub fn enqueue(&mut self, uid: thread::UId, storage: &mut super::ThreadMap<N>) -> Result<()> {
        self.queue
            .push_back(uid, storage)
            .map_err(|_| kerr!(InvalidArgument))
    }

    pub fn put(&mut self, uid: thread::UId, dt: u32) {
        if let Some(current) = self.current {
            if current == uid {
                self.current_left = self.current_left.saturating_sub(dt);
            }
        }
    }

    pub fn pick(&mut self, storage: &mut super::ThreadMap<N>) -> Option<(thread::UId, u32)> {
        match self.current {
            Some(current) if self.current_left > 0 => return Some((current, self.current_left)),
            Some(current) => {
                if self.queue.pop_front(storage).is_err() {
                    // If this happens, it means the internal state was corrupted.
                    // We cannot meaningfully continue, so we panic.
                    bug!("current thread not found in queue");
                }
                if self.queue.push_back(current, storage).is_err() {
                    // We popped the current thread from the queue, so it must be able to be pushed back.
                    // The scheduler is run exclusively so the space cannot be taken by another thread.
                    bug!("cannot push current thread back to queue");
                }

                self.current = self.queue.head();
                self.current_left = self.quantum;
            }
            None => {
                self.current = self.queue.head();
                self.current_left = self.quantum;
            }
        }

        self.current.map(|id| (id, self.current_left))
    }

    pub fn dequeue(&mut self, uid: thread::UId, storage: &mut super::ThreadMap<N>) -> Result<()> {
        self.queue.remove(uid, storage)?;

        if self.current == Some(uid) {
            self.current = None;
        }

        Ok(())
    }
}
