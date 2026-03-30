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
        traits::{Get, GetMut},
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
                if self.kill_task(task_id).is_err() {
                    // Should not be possible. The thread exists, so the task must exist.
                    bug!("failed to kill task {}", task_id);
                }
            }
        }
    }

    /// Triggers a reschedule at *latest* when we hit timepoint `next`.
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
            if self.rr_scheduler.enqueue(uid, &mut self.threads).is_err() {
                // This should not be possible.
                // - Thread is in the thread list.
                // - Thread is not linked into a different list.
                bug!("failed to enqueue thread {} into RR scheduler.", uid);
            }
        }
        reschedule();
        Ok(())
    }

    fn do_wakeups(&mut self, now: u64) {
        while let Some(uid) = self.wakeup.min() {
            let mut done = false;
            WaiterView::<N>::with(&mut self.threads, |view| {
                let waiter = view.get(uid).expect("THIS IS A BUG!");
                if waiter.until() > now {
                    Self::schedule_resched(now, waiter.until());
                    done = true;
                    return;
                }

                if let Err(_) = self.wakeup.remove(uid, view) {
                    bug!("failed to remove thread {} from wakeup tree.", uid);
                }
            });

            if done {
                break;
            }

            if self.enqueue(now, uid).is_err() {
                bug!("failed to enqueue thread {} after wakeup.", uid);
            }
        }
    }

    pub fn do_sched(&mut self, now: u64) -> Option<(*mut c_void, &mut task::Task)> {
        let dt = now - self.last_tick;
        self.last_tick = now;

        if let Some(old) = self.current {
            rt::ServerView::<N>::with(&mut self.threads, |view| {
                self.rt_scheduler.put(old, dt, view);
            });
            self.rr_scheduler.put(old, dt);
        }

        self.do_wakeups(now);

        let pick =
            rt::ServerView::<N>::with(&mut self.threads, |view| self.rt_scheduler.pick(now, view));
        let pick = pick.or_else(|| self.rr_scheduler.pick(&mut self.threads));
        let (new, budget) = pick.unwrap_or((thread::IDLE_THREAD, 1000));

        // At this point, the task/thread must exist. Everything else is a bug.
        let (ctx, task_id) = if let Some(thread) = self.threads.get(new) {
            (thread.ctx(), thread.task_id())
        } else {
            bug!("failed to pick thread {}. Does not exist.", new);
        };

        let task = if let Some(task) = self.tasks.get_mut(task_id) {
            task
        } else {
            bug!("failed to get task {}. Does not exist.", task_id);
        };

        // We don't need to resched if the thread has budget.
        self.current = Some(new);
        Self::schedule_resched(now, now.saturating_add(budget));
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
            // This should not be possible. The thread must exist since it's the current thread.
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
            // This should not be possible. The thread exists.
            bug!("failed to insert thread {} into wakeup tree.", uid);
        }

        self.dequeue(uid);
        reschedule();
        Ok(())
    }

    pub fn kick(&mut self, uid: thread::UId) -> Result<()> {
        WaiterView::<N>::with(&mut self.threads, |view| {
            self.wakeup.remove(uid, view)?;
            let thread = view.get_mut(uid).unwrap_or_else(|| {
                bug!("failed to get thread {} from wakeup tree.", uid);
            });
            thread.set_until(0);
            self.wakeup.insert(uid, view).unwrap_or_else(|_| {
                bug!("failed to re-insert thread {} into wakeup tree.", uid);
            });
            Ok(())
        })
    }

    pub fn dequeue(&mut self, uid: thread::UId) {
        rt::ServerView::<N>::with(&mut self.threads, |view| {
            self.rt_scheduler.dequeue(uid, view);
        });
        self.rr_scheduler.dequeue(uid, &mut self.threads);
    }

    pub fn create_task(&mut self, task: &task::Attributes) -> Result<task::UId> {
        let uid = task::UId::new(self.id_gen).ok_or(kerr!(InvalidArgument))?;
        self.id_gen += 1;

        self.tasks.insert(&uid, task::Task::new(uid, task)?)?;
        Ok(uid)
    }

    pub fn kill_task(&mut self, uid: task::UId) -> Result<()> {
        let task = self.tasks.get_mut(uid).ok_or(kerr!(InvalidArgument))?;

        while let Some(id) = task.threads().head() {
            // Borrow checker...
            rt::ServerView::<N>::with(&mut self.threads, |view| {
                self.rt_scheduler.dequeue(id, view);
            });
            self.rr_scheduler.dequeue(id, &mut self.threads);

            if task.threads_mut().remove(id, &mut self.threads).is_err() {
                // This should not be possible. The thread ID is from the thread list of the task, so it must exist.
                bug!("failed to remove thread {} from task {}.", id, uid);
            }
        }

        self.tasks.remove(&uid).ok_or(kerr!(InvalidArgument))?;
        Ok(())
    }

    pub fn create_thread(
        &mut self,
        task: Option<task::UId>,
        attrs: &thread::Attributes,
    ) -> Result<thread::UId> {
        let task = match task {
            Some(t) => t,
            None => self.current.ok_or(kerr!(InvalidArgument))?.owner(),
        };
        let task = self.tasks.get_mut(task).ok_or(kerr!(InvalidArgument))?;
        let uid = task.create_thread(self.id_gen, attrs, &mut self.threads)?;

        self.id_gen += 1;
        Ok(uid)
    }

    pub fn kill_thread(&mut self, uid: Option<thread::UId>) -> Result<()> {
        let uid = uid.unwrap_or(self.current.ok_or(kerr!(InvalidArgument))?);
        self.dequeue(uid);
        self.threads.remove(&uid).ok_or(kerr!(InvalidArgument))?;

        if Some(uid) == self.current {
            self.current = None;
            reschedule();
        }
        Ok(())
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
