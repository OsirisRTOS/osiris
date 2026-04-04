use crate::{
    sched::{
        ThreadMap,
        thread::{self},
    },
    types::{
        rbtree::RbTree,
        traits::{Get, GetMut},
        view::ViewMut,
    },
};

pub struct Scheduler<const N: usize> {
    edf: RbTree<thread::RtTree, thread::UId>,
}

pub type ServerView<'a, const N: usize> = ViewMut<'a, thread::UId, thread::RtServer, ThreadMap<N>>;

impl<const N: usize> Scheduler<N> {
    pub const fn new() -> Self {
        Self { edf: RbTree::new() }
    }

    pub fn enqueue(&mut self, uid: thread::UId, now: u64, storage: &mut ServerView<N>) {
        if let Some(server) = storage.get_mut(uid) {
            // Threads are only enqueued when they are runnable.
            server.on_wakeup(now);
            let _ = self.edf.insert(uid, storage);
        }
    }

    /// This should be called on each do_schedule call, to update the internal scheduler state.
    /// If this function returns Some(u64) it means the current thread has exhausted its budget and should be throttled until the returned timestamp.
    pub fn put(&mut self, uid: thread::UId, dt: u64, storage: &mut ServerView<N>) -> Option<u64> {
        if Some(uid) == self.edf.min() {
            if let Some(server) = storage.get_mut(uid) {
                return server.consume(dt);
            } else {
                bug!("thread {} not found in storage", uid);
            }
        }

        None
    }

    pub fn pick(&mut self, storage: &mut ServerView<N>) -> Option<(thread::UId, u32)> {
        self.edf
            .min()
            .and_then(|id| storage.get(id).map(|s| (id, s.budget())))
    }

    pub fn dequeue(&mut self, uid: thread::UId, storage: &mut ServerView<N>) {
        let _ = self.edf.remove(uid, storage);
    }
}
