use crate::{types::{rbtree::RbTree, traits::{Get, GetMut}, view::ViewMut}, sched::{ThreadMap, thread::{self}}};

pub struct Scheduler<const N: usize> {
    edf: RbTree<thread::RtTree, thread::UId>,
}

pub type ServerView<'a, const N: usize> = ViewMut<'a, thread::UId, thread::RtServer, ThreadMap<N>>;

impl<const N: usize> Scheduler<N> {
    pub const fn new() -> Self {
        Self {
            edf: RbTree::new(),
        }
    }

    pub fn enqueue(&mut self, uid: thread::UId, storage: &mut ServerView<N>) {
        self.edf.insert(uid, storage);
    }

    pub fn put(&mut self, uid: thread::UId, dt: u64, storage: &mut ServerView<N>) {
        if let Some(server) = storage.get_mut(uid) {
            server.consume(dt);
        }
    }

    pub fn pick(&mut self, now: u64, storage: &mut ServerView<N>) -> Option<(thread::UId, u64)> {
        let id = self.edf.min()?;
        
        if storage.get(id)?.budget() == 0 {
            self.edf.remove(id, storage);
            storage.get_mut(id)?.replenish(now);
            self.edf.insert(id, storage);
        }

        // Insert updated the min cache.
        self.edf.min().and_then(|id| storage.get(id).map(|s| (id, s.budget())))
    }

    pub fn dequeue(&mut self, uid: thread::UId, storage: &mut ServerView<N>) {
        self.edf.remove(uid, storage);
    }
}