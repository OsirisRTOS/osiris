//! This module provides access to the scheduler.

mod dispch;
pub mod rr;
pub mod rt;
pub mod task;
pub mod thread;

use core::{
    ffi::c_void,
    sync::atomic::{AtomicBool, Ordering},
};

use hal::Schedable;

use crate::{
    error::Result,
    mem,
    sched::thread::Waiter,
    sync::{self, atomic::AtomicU64, spinlock::SpinLocked},
    time::{self},
    types::{
        array::IndexMap,
        rbtree::RbTree,
        traits::{Get, GetMut, Project},
        view::ViewMut,
    },
};

type ThreadMap<const N: usize> = IndexMap<thread::UId, thread::Thread, N>;
type TaskMap<const N: usize> = IndexMap<task::UId, task::Task, N>;

type GlobalScheduler = Scheduler<32>;

static SCHED: SpinLocked<GlobalScheduler> = SpinLocked::new(GlobalScheduler::new());

static DISABLED: AtomicBool = AtomicBool::new(true);
static NEXT_TICK: AtomicU64 = AtomicU64::new(0);

type WaiterView<'a, const N: usize> = ViewMut<'a, thread::UId, thread::Waiter, ThreadMap<N>>;

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

    fn land(&mut self, ctx: *mut c_void) {
        if let Some(current) = self.current {
            let mut kill = None;
            if let Some(thread) = self.threads.get_mut(current) {
                if thread.save_ctx(ctx).is_err() {
                    warn!(
                        "failed to save context (SP: {:x}) of thread {}.",
                        ctx as usize, current
                    );
                    kill = Some(thread.task_id());
                }
            } else {
                bug!("failed to land thread {}. Does not exist.", current);
            }

            if let Some(task_id) = kill {
                self.dequeue(current);
                self.current = None;
                self.kill_task(task_id);
            }
        }
    }

    fn schedule_resched(now: u64, next: u64) {
        let old = NEXT_TICK.load(Ordering::Acquire);

        if old > now && old <= next {
            return;
        }

        NEXT_TICK.store(next, Ordering::Release);
    }

    pub fn enqueue(&mut self, now: u64, uid: thread::UId) -> Result<()> {
        let thread = self.threads.get(uid).ok_or(kerr!(InvalidArgument))?;

        if thread.rt_server().is_some() {
            let mut view = rt::ServerView::<N>::new(&mut self.threads);
            self.rt_scheduler.enqueue(uid, now, &mut view);
        } else {
            self.rr_scheduler.enqueue(uid, &mut self.threads)?;
        }
        reschedule();
        Ok(())
    }

    fn do_wakeups(&mut self, now: u64) {
        while let Some(uid) = self.wakeup.min() {
            {
                let mut view = WaiterView::<N>::new(&mut self.threads);
                let waiter = view.get(uid).expect("THIS IS A BUG!");

                if waiter.until() > now {
                    Self::schedule_resched(now, waiter.until());
                    break;
                }

                self.wakeup.remove(uid, &mut view);
            }

            self.enqueue(now, uid);
        }
    }

    pub fn do_sched(&mut self, now: u64) -> Option<(*mut c_void, &mut task::Task)> {
        let dt = now - self.last_tick;
        self.last_tick = now;

        if let Some(old) = self.current {
            let mut view = rt::ServerView::<N>::new(&mut self.threads);
            self.rt_scheduler.put(old, dt, &mut view);
            self.rr_scheduler.put(old, dt);
        }

        self.do_wakeups(now);

        let mut view = rt::ServerView::<N>::new(&mut self.threads);

        let (new, budget) = self
            .rt_scheduler
            .pick(now, &mut view)
            .or_else(|| self.rr_scheduler.pick(&mut self.threads))
            .unwrap_or((thread::IDLE_THREAD, 1000));

        let ctx = self.threads.get(new)?.ctx();
        let task = self.tasks.get_mut(self.threads.get(new)?.task_id())?;

        self.current = Some(new);
        let next = now.saturating_add(budget);

        Self::schedule_resched(now, next);
        Some((ctx, task))
    }

    pub fn sleep_until(&mut self, until: u64, now: u64) -> Result<()> {
        if until <= now {
            return Ok(());
        }
        let uid = self.current.ok_or(kerr!(InvalidArgument))?;

        if let Some(thread) = self.threads.get_mut(uid) {
            thread.set_waiter(Some(Waiter::new(until, uid)));
        } else {
            bug!(
                "failed to put current thread {} to sleep. Does not exist.",
                uid
            );
        }

        if self
            .wakeup
            .insert(uid, &mut WaiterView::<N>::new(&mut self.threads))
            .is_err()
        {
            bug!("failed to insert thread {} into wakeup tree.", uid);
        }



        self.dequeue(uid);
        reschedule();
        Ok(())
    }

    pub fn kick(&mut self, uid: thread::UId) -> Result<()> {
        let thread = self.threads.get_mut(uid).ok_or(kerr!(InvalidArgument))?;
        if let Some(waiter) = Project::<Waiter>::project_mut(thread) {
            waiter.set_until(0);
        }
        Ok(())
    }

    pub fn dequeue(&mut self, uid: thread::UId) {
        let mut view = rt::ServerView::<N>::new(&mut self.threads);
        self.rt_scheduler.dequeue(uid, &mut view);
        self.rr_scheduler.dequeue(uid, &mut self.threads);
    }

    pub fn create_task(&mut self, task: &task::Attributes) -> Result<task::UId> {
        let uid = task::UId::new(self.id_gen).ok_or(kerr!(InvalidArgument))?;
        self.id_gen += 1;

        self.tasks.insert(&uid, task::Task::new(uid, task)?)?;
        Ok(uid)
    }

    pub fn kill_task(&mut self, uid: task::UId) -> Result<()> {
        let task_id = self.tasks.get(uid).ok_or(kerr!(InvalidArgument))?.id;
        self.tasks.remove(&uid).ok_or(kerr!(InvalidArgument))?;

        let begin = match self.threads.next(None) {
            Some(i) => i,
            None => return Ok(()),
        };
        let mut i = begin;

        while i != begin {
            i = (i + 1) % N;

            let mut id = None;
            if let Some(thread) = self.threads.at_cont(i) {
                if thread.task_id() == task_id {
                    id = Some(thread.uid());
                }
            }

            if let Some(id) = id {
                self.dequeue(id);
            }
        }

        Ok(())
    }

    pub fn create_thread(
        &mut self,
        task: task::UId,
        attrs: &thread::Attributes,
    ) -> Result<thread::UId> {
        let task = self.tasks.get_mut(task).ok_or(kerr!(InvalidArgument))?;
        let thread = task.create_thread(self.id_gen, attrs)?;
        let uid = thread.uid();

        self.threads.insert(&uid, thread)?;

        self.id_gen += 1;
        Ok(uid)
    }
}

pub fn with<T, F: FnOnce(&mut GlobalScheduler) -> T>(f: F) -> T {
    sync::atomic::irq_free(|| {
        let mut sched = SCHED.lock();
        f(&mut sched)
    })
}

pub fn init(kaddr_space: mem::vmm::AddressSpace) {
    with(|sched| {
        let uid = task::KERNEL_TASK;
        if let Ok(task) = task::Task::from_addr_space(uid, kaddr_space) {
            if sched.tasks.insert(&uid, task).is_err() {
                panic!("failed to create kernel task.");
            }
        } else {
            panic!("failed to create kernel address space.");
        }
    })
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
    if DISABLED.load(Ordering::Acquire) {
        return;
    }

    hal::Machine::trigger_reschedule();
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn sched_enter(mut ctx: *mut c_void) -> *mut c_void {
    with(|sched| {
        let old = sched.current.map(|c| c.owner());
        sched.land(ctx);

        if let Some((new, task)) = sched.do_sched(time::tick()) {
            if old != Some(task.id) {
                dispch::prepare(task);
            }
            ctx = new;
        } else {
            bug!("failed to schedule a thread. No threads available.");
        }

        ctx
    })
}
