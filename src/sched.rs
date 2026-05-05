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

use crate::hal::{self, Schedable};

use crate::{
    error::Result,
    mem,
    sched::thread::Waiter,
    sync::{self, atomic::AtomicU64, spinlock::SpinLocked},
    time::{self},
    types::{
        array::BitReclaimMap,
        rbtree::RbTree,
        traits::{Get, GetMut},
        view::ViewMut,
    },
};

type ThreadMap<const N: usize, const WORDS: usize> =
    BitReclaimMap<thread::UId, thread::Thread, N, WORDS>;
type TaskMap<const N: usize, const WORDS: usize> = BitReclaimMap<task::UId, task::Task, N, WORDS>;

const THREAD_COUNT: usize = 32;
type GlobalScheduler = Scheduler<THREAD_COUNT, { THREAD_COUNT.div_ceil(usize::BITS as usize) }>;

static SCHED: SpinLocked<GlobalScheduler> = SpinLocked::new(GlobalScheduler::new());

static DISABLED: AtomicBool = AtomicBool::new(true);
static NEXT_TICK: AtomicU64 = AtomicU64::new(0);

type WaiterView<'a, const N: usize, const WORDS: usize> =
    ViewMut<'a, thread::UId, thread::Waiter, ThreadMap<N, WORDS>>;

pub struct Scheduler<const N: usize, const WORDS: usize> {
    threads: ThreadMap<N, WORDS>,
    tasks: TaskMap<N, WORDS>,

    rt_scheduler: rt::Scheduler<N, WORDS>,
    rr_scheduler: rr::Scheduler<N, WORDS>,

    wakeup: RbTree<thread::WakupTree, thread::UId>,

    current: Option<thread::UId>,
    last_tick: u64,
}

// Safety: The scheduler is not Copy or Clone.
// The scheduler owns all its data exclusively.
unsafe impl<const N: usize, const WORDS: usize> Send for Scheduler<N, WORDS> {}
// Safety: The scheduler does only allow access to its data through &mut self, which is synchronized by the SCHED spinlock.
unsafe impl<const N: usize, const WORDS: usize> Sync for Scheduler<N, WORDS> {}

/// We define dequeue as a macro in order to avoid borrow checker issues.
macro_rules! dequeue {
    ($self:expr, $uid:expr) => {
        rt::ServerView::<N, WORDS>::with(&mut $self.threads, |view| {
            $self.rt_scheduler.dequeue($uid, view)
        })
        .or_else(|_| $self.rr_scheduler.dequeue($uid, &mut $self.threads))
        .or_else(|_| {
            $self
                .wakeup
                .remove($uid, &mut WaiterView::<N, WORDS>::new(&mut $self.threads))
        })
    };
}

impl<const N: usize, const WORDS: usize> Scheduler<N, WORDS> {
    const fn new() -> Self {
        Self {
            threads: ThreadMap::new(),
            tasks: TaskMap::new(),
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
                if self.kill_by_task(task_id).is_err() {
                    // Should not be possible. The thread exists, so the task must exist.
                    bug!("failed to kill task {}", task_id);
                }
            }
        }
    }

    /// Triggers a reschedule at *latest* when we hit timepoint `next`.
    /// Note that we may reschedule earlier than `next` if another thread wakes up or is enqueued, but we will never reschedule later than `next`.
    ///
    /// `now` - The current timepoint, in ticks.
    /// `next` - The next timepoint to reschedule at, in ticks.
    fn next_resched(now: u64, next: u64) {
        let old = NEXT_TICK.load(Ordering::Acquire);

        if old > now && old <= next {
            return;
        }

        NEXT_TICK.store(next, Ordering::Release);
    }

    /// Enqueues a thread into the scheduler. This will trigger a reschedule.
    ///
    /// `uid` - The UID of the thread to enqueue.
    /// `now` - The current timepoint, in ticks. This is used for RT threads to calculate their deadlines.
    ///
    /// Returns an error if the thread does not exist.
    pub fn enqueue(&mut self, now: u64, uid: thread::UId) -> Result<()> {
        let thread = self.threads.get(uid).ok_or(kerr!(InvalidArgument))?;

        if thread.rt_server().is_some() {
            let mut view = rt::ServerView::<N, WORDS>::new(&mut self.threads);
            self.rt_scheduler.enqueue(uid, now, &mut view)?;
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
            WaiterView::<N, WORDS>::with(&mut self.threads, |view| {
                if let Some(waiter) = view.get(uid) {
                    if waiter.until() > now {
                        Self::next_resched(now, waiter.until());
                        done = true;
                        return;
                    }

                    if let Err(_) = self.wakeup.remove(uid, view) {
                        bug!("failed to remove thread {} from wakeup tree.", uid);
                    }
                } else {
                    bug!("failed to get thread {} from wakeup tree.", uid);
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

    /// Syncs the new state after the last do_sched call to the scheduler, and returns whether we need to immediately reschedule.
    fn sync_to_sched(&mut self, now: u64) -> bool {
        let dt = now - self.last_tick;
        self.last_tick = now;

        if let Some(old) = self.current {
            let throttle = rt::ServerView::<N, WORDS>::with(&mut self.threads, |view| {
                self.rt_scheduler.put(old, dt, view)
            });

            if let Some(throttle) = throttle {
                let _ = self.sleep_until(throttle, now);
                return true;
            }

            self.rr_scheduler.put(old, dt as u32);
        }

        self.do_wakeups(now);
        false
    }

    fn select_next(&mut self) -> (thread::UId, u32) {
        rt::ServerView::<N, WORDS>::with(&mut self.threads, |view| self.rt_scheduler.pick(view))
            .or_else(|| self.rr_scheduler.pick(&mut self.threads))
            .unwrap_or((thread::IDLE_THREAD, 1000))
    }

    /// Picks the next thread to run and returns its context and task. This should only be called by sched_enter after land.
    fn do_sched(&mut self, now: u64) -> Option<(*mut c_void, &mut task::Task)> {
        // Sync the new state to the scheduler.
        if self.sync_to_sched(now) {
            // Trigger reschedule after interrupts are enabled.
            return None;
        }

        // Pick the next thread to run.
        let (new, budget) = self.select_next();

        // At this point, the task/thread must exist. Everything else is a bug.
        let Some(thread) = self.threads.get(new) else {
            bug!("failed to pick thread {}. Does not exist.", new);
        };
        let (ctx, task_id) = (thread.ctx(), thread.task_id());

        let Some(task) = self.tasks.get_mut(task_id) else {
            bug!("failed to get task {}. Does not exist.", task_id);
        };

        // We don't need to resched if the thread has budget.
        self.current = Some(new);
        Self::next_resched(now, now.saturating_add(budget as u64));
        Some((ctx, task))
    }

    /// Puts the current thread to sleep until the specified timepoint. This will trigger a reschedule.
    ///
    /// `until` - The timepoint to sleep until, in ticks. This is an absolute time, not a relative time.
    /// `now` - The current timepoint, in ticks.
    ///
    /// Returns an error if there is no current thread, it is not enqueued, or if the specified timepoint is in the past.
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

        dequeue!(self, uid)?;

        if self
            .wakeup
            .insert(uid, &mut WaiterView::<N, WORDS>::new(&mut self.threads))
            .is_err()
        {
            // This should not be possible. The thread exists.
            bug!("failed to insert thread {} into wakeup tree.", uid);
        }

        reschedule();
        Ok(())
    }

    /// If the thread is currently sleeping, this will trigger a wakeup on the next reschedule. Note this does not trigger an immediate reschedule.
    ///
    /// Returns an error if the thread does not exist, or if the thread is not currently sleeping.
    #[allow(dead_code)]
    pub fn kick(&mut self, uid: thread::UId) -> Result<()> {
        WaiterView::<N, WORDS>::with(&mut self.threads, |view| {
            self.wakeup.remove(uid, view)?;
            let thread = view.get_mut(uid).unwrap_or_else(|| {
                // This should not be possible. The thread must exist since it's in the wakeup tree.
                bug!("failed to get thread {} from wakeup tree.", uid);
            });
            thread.set_until(0);
            self.wakeup.insert(uid, view).unwrap_or_else(|_| {
                // This should not be possible. The thread exists and we just removed it from the wakeup tree, so it must be able to be re-inserted.
                bug!("failed to re-insert thread {} into wakeup tree.", uid);
            });
            Ok(())
        })
    }

    /// This will just remove the thread from the scheduler, but it will not trigger a reschedule, even if the thread is currently running.
    ///
    /// Returns an error if the thread does not exist, or if the thread is not currently enqueued in any scheduler.
    pub fn dequeue(&mut self, uid: thread::UId) -> Result<()> {
        dequeue!(self, uid)
    }

    pub fn create_task(&mut self, attrs: task::Attributes) -> Result<task::UId> {
        self.tasks.insert_with(|idx| {
            let task = task::Task::new(task::UId::new(idx), attrs);
            task.map(|t| (task::UId::new(idx), t))
        })
    }

    /// Dequeues all threads of the task and removes the task. If the current thread belongs to the task, reschedule will be triggered.
    ///
    /// If the task does not exist, an error will be returned.
    pub fn kill_by_task(&mut self, uid: task::UId) -> Result<()> {
        let task = self.tasks.get_mut(uid).ok_or(kerr!(InvalidArgument))?;

        while let Some(id) = task.threads().head() {
            dequeue!(self, id)?;

            if task.threads_mut().remove(id, &mut self.threads).is_err() {
                // This should not be possible. The thread ID is from the thread list of the task, so it must exist.
                bug!("failed to remove thread {} from task {}.", id, uid);
            }

            if self.threads.remove(&id).is_none() {
                // This should not be possible. The thread ID is from the thread list of the task, so it must exist.
                bug!("failed to remove thread {} from thread list.", id);
            }

            if Some(id) == self.current {
                self.current = None;
                reschedule();
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

        self.threads
            .insert_with(|idx| {
                let uid = task.allocate_tid().get_uid(idx);
                let stack = task.allocate_stack(attrs)?;
                let thread = thread::Thread::new(uid, stack, attrs.attrs);
                Ok((uid, thread))
            })
            .and_then(|k| {
                task.register_thread(k, &mut self.threads)?;
                Ok(k)
            })
    }

    /// Dequeues a thread and removes it from its corresponding task. If the thread is currently running, reschedule will be triggered.
    ///
    /// `uid` - The UID of the thread to kill, or None to kill the current thread.
    ///
    /// If the thread does not exist, or if `uid` is None and there is no current thread, an error will be returned.
    pub fn kill_by_thread(&mut self, uid: Option<thread::UId>) -> Result<()> {
        let uid = uid.unwrap_or(self.current.ok_or(kerr!(InvalidArgument))?);
        self.dequeue(uid)?;

        self.tasks
            .get_mut(uid.tid().owner())
            .ok_or(kerr!(InvalidArgument))?
            .threads_mut()
            .remove(uid, &mut self.threads)?;

        self.threads.remove(&uid).ok_or(kerr!(InvalidArgument))?;

        if Some(uid) == self.current {
            self.current = None;
            reschedule();
        }
        Ok(())
    }
}

/// This function provides safe access to the global scheduler.
/// It disables interrupts and locks the scheduler. Use with caution!
pub fn with<T, F: FnOnce(&mut GlobalScheduler) -> T>(f: F) -> T {
    sync::atomic::irq_free(|| {
        let mut sched = SCHED.lock();
        f(&mut sched)
    })
}

/// Initializes the scheduler. This should be called once during kernel initialization, before any threads are created.
///
/// `kaddr_space` - The address space of the kernel task. This is used to create the kernel task, which is required for the scheduler to function.
///
/// If the kernel task cannot be created, this function will panic. Note that the kernel task is essential for the system to function, so we cannot continue without it.
pub fn init(kaddr_space: mem::vmm::AddressSpace) {
    with(|sched| {
        let attrs = task::Attributes {
            resrv_pgs: None,
            address_space: Some(kaddr_space),
        };

        sched.create_task(attrs).unwrap_or_else(|e| {
            panic!("failed to create kernel task: {}", e);
        });
    })
}

/// This should be called on each timer tick, and if it returns true, sched_enter should be called to reschedule.
///
/// `now` - The current timepoint, in ticks.
pub fn needs_reschedule(now: u64) -> bool {
    if DISABLED.load(Ordering::Acquire) {
        return false;
    }

    now >= NEXT_TICK.load(Ordering::Acquire)
}

/// This will disable rescheduling until the next call to enable. Use with caution!
#[inline]
#[allow(dead_code)]
pub fn disable() {
    DISABLED.store(true, Ordering::Release);
}

#[inline]
pub fn enable() {
    DISABLED.store(false, Ordering::Release);
}

/// Triggers a reschedule immediately, when interrupts are enabled.
/// This must be called after enqueueing a thread, or after waking up a thread, or putting the current thread to sleep.
pub fn reschedule() {
    if DISABLED.load(Ordering::Acquire) {
        return;
    }

    hal::Machine::trigger_reschedule();
}

/// This will be called by the architecture-specific code to enter the scheduler. It will land the current thread, pick the next thread to run, and return its context and task.
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
        }

        ctx
    })
}
