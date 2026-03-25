//! This module provides access to the scheduler.

mod dispch;
pub mod rt;
pub mod task;
pub mod thread;

use core::ffi::c_void;

use hal::Schedable;

use crate::{
    mem, sync::spinlock::SpinLocked, types::{
        array::IndexMap,
        rbtree::RbTree,
        traits::{Get, GetMut},
        view::ViewMut,
    }, utils::KernelError
};

type ThreadMap<const N: usize> = IndexMap<thread::UId, thread::Thread, N>;
type TaskMap<const N: usize> = IndexMap<task::UId, task::Task, N>;

static SCHED: SpinLocked<Scheduler<32>> = SpinLocked::new(Scheduler::new());

pub struct Scheduler<const N: usize> {
    threads: ThreadMap<N>,
    tasks: TaskMap<N>,
    id_gen: usize,

    rt_scheduler: rt::Scheduler<N>,

    wakeup: RbTree<thread::WakupTree, thread::UId>,

    current: thread::UId,
    last_tick: u64,
    next_tick: u64,
}

impl<const N: usize> Scheduler<N> {
    pub const fn new() -> Self {
        Self {
            threads: IndexMap::new(),
            tasks: IndexMap::new(),
            id_gen: 1,
            rt_scheduler: rt::Scheduler::new(),
            wakeup: RbTree::new(),
            current: thread::IDLE_THREAD,
            last_tick: 0,
            next_tick: 0,
        }
    }

    fn land(&mut self, ctx: *mut c_void) -> Result<(), KernelError> {
        // A thread must not disappear while it is running.
        let current = self.threads.get_mut(self.current).ok_or(KernelError::InvalidArgument)?;
        // The context pointer must not be bogus after a sched_enter.
        current.save_ctx(ctx)
    }

    pub fn enqueue(&mut self, uid: thread::UId) -> Result<(), KernelError> {
        let thread = self.threads.get(uid).ok_or(KernelError::InvalidArgument)?;

        if thread.rt_server().is_some() {
            let mut view =
                ViewMut::<thread::UId, thread::RtServer, ThreadMap<N>>::new(&mut self.threads);
            self.rt_scheduler.enqueue(uid, &mut view);
        }

        Ok(())
    }

    pub fn do_sched(
        &mut self,
        now: u64,
        old: Option<thread::UId>,
    ) -> Option<(*mut c_void, &mut task::Task)> {
        let dt = now - self.last_tick;
        self.last_tick = now;

        if let Some(old) = old {
            let mut view = rt::ServerView::<N>::new(&mut self.threads);
            // If this is not a real-time thread, this will just do nothing.
            self.rt_scheduler.put(old, dt, &mut view);

            // TODO: thread is still enqueued. Dequeue if blocked or sleeping and put to the respective tree/list.
            // If it exited remove it completely.
        }

        let mut view = rt::ServerView::<N>::new(&mut self.threads);
        let (new, budget) = self.rt_scheduler.pick(now, &mut view)?;

        let ctx = self.threads.get(new)?.ctx();
        let task = self.tasks.get_mut(self.threads.get(new)?.task_id())?;

        self.current = new;
        self.next_tick = now + budget;
        Some((ctx, task))
    }

    pub fn dequeue(&mut self, uid: thread::UId) -> Option<thread::Thread> {
        let mut view = rt::ServerView::<N>::new(&mut self.threads);
        // If this is not a real-time thread, this will just do nothing.
        self.rt_scheduler.dequeue(uid, &mut view);

        self.threads.remove(&uid)
    }

    pub fn create_task(&mut self, task: &task::Attributes) -> Result<task::UId, KernelError> {
        let uid = task::UId::new(self.id_gen).ok_or(KernelError::InvalidArgument)?;
        self.id_gen += 1;

        self.tasks.insert(&uid, task::Task::new(uid, task)?);
        Ok(uid)
    }

    pub fn create_thread(&mut self, task: task::UId, attrs: &thread::Attributes) -> Result<thread::UId, KernelError> {
        let task = self.tasks.get_mut(task).ok_or(KernelError::InvalidArgument)?;
        let thread = task.create_thread(self.id_gen, attrs)?;
        let uid = thread.uid();
        self.id_gen += 1;
        Ok(uid)
    }
}

pub fn init(kaddr_space: mem::vmm::AddressSpace) {
    let mut sched = SCHED.lock();
    let uid = task::KERNEL_TASK;
    sched.tasks.insert(&uid, task::Task::from_addr_space(uid, kaddr_space));
}

pub fn needs_reschedule(now: u64) -> bool {
    let sched = SCHED.lock();
    now >= sched.next_tick
}

pub fn create_task(attrs: &task::Attributes) -> Result<task::UId, KernelError> {
    SCHED.lock().create_task(attrs)
}

pub fn create_thread(task: task::UId, attrs: &thread::Attributes) -> Result<thread::UId, KernelError> {
    SCHED.lock().create_thread(task, attrs)
}

/// Reschedule the tasks.
pub fn reschedule() {
    hal::Machine::trigger_reschedule();
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn sched_enter(ctx: *mut c_void) -> *mut c_void {
    let mut sched = SCHED.lock();
    let mut broken = false;
    let old = sched.current;

    if sched.land(ctx).is_err() {
        if sched.current == thread::IDLE_THREAD {
            BUG!("failed to land the idle thread. something is horribly broken.");
        }

        // If we cannot reasonably land. We dequeue the thread.
        sched.dequeue(old);
        // TODO: Warn
        sched.current = thread::IDLE_THREAD;
        broken = true;
    }

    let now = 0;

    if let Some((ctx, task)) = sched.do_sched(now, Some(old)) {
        if task.id != old.owner() {
            dispch::prepare(task);
        }
        ctx
    } else if broken {
        BUG!("failed to reschedule after a failed landing. something is horribly broken.");
    } else {
        ctx
    }
}
