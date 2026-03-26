//! This module provides access to the scheduler.

mod dispch;
pub mod rt;
pub mod rr;
pub mod task;
pub mod thread;

use core::{ffi::c_void, sync::atomic::{AtomicBool, Ordering}};

use hal::Schedable;

use crate::{
    mem, sync::{atomic::AtomicU64, spinlock::SpinLocked}, time::{self, tick}, types::{
        array::IndexMap,
        rbtree::RbTree,
        traits::{Get, GetMut},
        view::ViewMut,
    }, utils::KernelError
};

type ThreadMap<const N: usize> = IndexMap<thread::UId, thread::Thread, N>;
type TaskMap<const N: usize> = IndexMap<task::UId, task::Task, N>;

static SCHED: SpinLocked<Scheduler<32>> = SpinLocked::new(Scheduler::new());

static DISABLED: AtomicBool = AtomicBool::new(true);
static NEXT_TICK: AtomicU64 = AtomicU64::new(0);

pub struct Scheduler<const N: usize> {
    threads: ThreadMap<N>,
    tasks: TaskMap<N>,
    id_gen: usize,

    rt_scheduler: rt::Scheduler<N>,
    rr_scheduler: rr::Scheduler<N>,

    wakeup: RbTree<thread::WakupTree, thread::UId>,

    current: Option<thread::UId>,
    last_tick: u64,
}

impl<const N: usize> Scheduler<N> {
    pub const fn new() -> Self {
        Self {
            threads: IndexMap::new(),
            tasks: IndexMap::new(),
            id_gen: 1,
            rt_scheduler: rt::Scheduler::new(),
            rr_scheduler: rr::Scheduler::new(),
            wakeup: RbTree::new(),
            current: None,
            last_tick: 0,
        }
    }

    fn land(&mut self, ctx: *mut c_void) -> Result<(), KernelError> {
        if let Some(current) = self.current {
            let thread = self.threads.get_mut(current).ok_or(KernelError::InvalidArgument)?;
            return thread.save_ctx(ctx);
        }

        Ok(())
    }

    pub fn enqueue(&mut self, uid: thread::UId) -> Result<(), KernelError> {
        let thread = self.threads.get(uid).ok_or(KernelError::InvalidArgument)?;

        if thread.rt_server().is_some() {
            let mut view =
                ViewMut::<thread::UId, thread::RtServer, ThreadMap<N>>::new(&mut self.threads);
            self.rt_scheduler.enqueue(uid, &mut view);
        } else {
            self.rr_scheduler.enqueue(uid, &mut self.threads)?;
        }

        // A new thread was added -> Trigger a reschedule.
        NEXT_TICK.store(tick(), Ordering::Release);
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
            // If this is not a round-robin thread, this will just do nothing.
            self.rr_scheduler.put(old, dt);

            // TODO: thread is still enqueued. Dequeue if blocked or sleeping and put to the respective tree/list.
            // If it exited remove it completely.
        }

        let mut view = rt::ServerView::<N>::new(&mut self.threads);

        let (new, budget) = if let Some((new, budget)) = self.rt_scheduler.pick(now, &mut view) {
            (new, budget)
        } else if let Some((new, budget)) = self.rr_scheduler.pick(&mut self.threads) {
            (new, budget)
        } else {
            // No thread to run. Run the idle thread.
            (thread::IDLE_THREAD, u64::MAX)
        };

        let ctx = self.threads.get(new)?.ctx();
        let task = self.tasks.get_mut(self.threads.get(new)?.task_id())?;

        self.current = Some(new);

        // Only store next_tick if now + budget is smaller than the current next tick.
        let next_tick = now + budget;
        let mut old_tick = NEXT_TICK.load(Ordering::Acquire);

        while NEXT_TICK.compare_exchange(old_tick, next_tick, Ordering::Release, Ordering::Acquire).is_err() {
            old_tick = NEXT_TICK.load(Ordering::Acquire);
            if next_tick >= old_tick {
                break;
            }
        }

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

        self.tasks.insert(&uid, task::Task::new(uid, task)?)?;
        Ok(uid)
    }

    pub fn create_thread(&mut self, task: task::UId, attrs: &thread::Attributes) -> Result<thread::UId, KernelError> {
        let task = self.tasks.get_mut(task).ok_or(KernelError::InvalidArgument)?;
        let thread = task.create_thread(self.id_gen, attrs)?;
        let uid = thread.uid();

        self.threads.insert(&uid, thread)?;

        self.id_gen += 1;
        Ok(uid)
    }
}

pub fn init(kaddr_space: mem::vmm::AddressSpace) -> Result<(), KernelError> {
    let mut sched = SCHED.lock();
    let uid = task::KERNEL_TASK;
    sched.tasks.insert(&uid, task::Task::from_addr_space(uid, kaddr_space)?)
}

pub fn create_task(attrs: &task::Attributes) -> Result<task::UId, KernelError> {
    SCHED.lock().create_task(attrs)
}

pub fn create_thread(task: task::UId, attrs: &thread::Attributes) -> Result<thread::UId, KernelError> {
    let mut sched = SCHED.lock();
    sched.create_thread(task, attrs)
}

pub fn enqueue(uid: thread::UId) -> Result<(), KernelError> {
    SCHED.lock().enqueue(uid)
}

pub fn needs_reschedule(now: u64) -> bool {
    if DISABLED.load(Ordering::Acquire) {
        return false;
    }

    now >= NEXT_TICK.load(Ordering::Acquire)
}

#[inline]
pub fn disable() {
    DISABLED.store(true, Ordering::Release);
}

#[inline]
pub fn enable() {
    DISABLED.store(false, Ordering::Release);
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
        sched.current.inspect(|uid| {
            if *uid == thread::IDLE_THREAD {
                BUG!("failed to land the idle thread. something is horribly broken.");
            }

            // If we cannot reasonably land. We dequeue the thread.
            sched.dequeue(*uid);
            // TODO: Warn
            sched.current = None;
            broken = true;
        });
    }

    if let Some((ctx, task)) = sched.do_sched(time::tick(), old) {
        if let Some(old) = old
            && task.id != old.owner() {
                dispch::prepare(task);
            }
        
        ctx
    } else if broken {
        BUG!("failed to reschedule after a failed landing. something is horribly broken.");
    } else {
        ctx
    }
}
