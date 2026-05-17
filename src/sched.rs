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
    sync::{self, atomic::AtomicU64, spinlock::SpinLocked},
    time::{self},
    types::{
        array::BitReclaimMap,
        rbtree::RbTree,
        traits::{Get, GetMut},
        view::ViewMut,
    },
};

type ThreadMap<const N: usize> = BitReclaimMap<thread::UId, thread::Thread, N>;
type TaskMap<const N: usize> = BitReclaimMap<task::UId, task::Task, N>;

pub(crate) const THREAD_COUNT: usize = 32;
type GlobalScheduler = Scheduler<THREAD_COUNT>;

static SCHED: SpinLocked<GlobalScheduler> = SpinLocked::new(GlobalScheduler::new());

static DISABLED: AtomicBool = AtomicBool::new(true);
static NEXT_TICK: AtomicU64 = AtomicU64::new(0);

type WaiterView<'a, const N: usize> = ViewMut<'a, thread::UId, thread::Waiter, ThreadMap<N>>;

pub struct Scheduler<const N: usize> {
    threads: ThreadMap<N>,
    tasks: TaskMap<N>,

    rt_scheduler: rt::Scheduler<N>,
    rr_scheduler: rr::Scheduler<N>,

    wakeup: RbTree<thread::WakupTree, thread::UId>,

    current: Option<thread::UId>,
    last_tick: u64,
}

// Safety: The scheduler is not Copy or Clone.
// The scheduler owns all its data exclusively.
unsafe impl<const N: usize> Send for Scheduler<N> {}
// Safety: The scheduler does only allow access to its data through &mut self, which is synchronized by the SCHED spinlock.
unsafe impl<const N: usize> Sync for Scheduler<N> {}

/// We define kill as a macro in order to avoid borrow checker issues.
macro_rules! kill {
    ($self:expr, $uid:expr) => {{
        let _ = rt::ServerView::<N>::with(&mut $self.threads, |view| {
            $self.rt_scheduler.dequeue($uid, view)
        });
        let _ = $self.rr_scheduler.dequeue($uid, &mut $self.threads);
        let _ = $self
            .wakeup
            .remove($uid, &mut WaiterView::<N>::new(&mut $self.threads));
        if let Some(thread) = $self.threads.get_mut($uid) {
            thread.resume();
        }
        Ok::<(), crate::error::Error>(())
    }};
}

impl<const N: usize> Scheduler<N> {
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
        let thread = self.threads.get(uid).ok_or(kerr!(EINVAL))?;

        if thread.rt_server().is_some() {
            let mut view = rt::ServerView::<N>::new(&mut self.threads);
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
            let mut stop = false;
            WaiterView::<N>::with(&mut self.threads, |view| {
                if let Some(waiter) = view.get(uid) {
                    if waiter.until() > now {
                        Self::next_resched(now, waiter.until());
                        stop = true;
                        return;
                    }

                    if let Err(_) = self.wakeup.remove(uid, view) {
                        bug!("failed to remove thread {} from wakeup tree.", uid);
                    }
                } else {
                    bug!("failed to get thread {} from wakeup tree.", uid);
                }
            });

            if stop {
                break;
            }

            if let Some(thread) = self.threads.get_mut(uid) {
                thread.resume();
            } else {
                // This should not be possible. The thread is in the wakeup tree, so it must exist.
                bug!("failed to wake thread {}. Does not exist.", uid);
            }

            if self.enqueue(now, uid).is_err() {
                bug!("failed to enqueue thread {} after wakeup.", uid);
            }
        }
    }

    /// Syncs the scheduler state at the beginning of a reschedule.
    fn sync_to_sched(&mut self, now: u64) {
        let dt = now - self.last_tick;
        self.last_tick = now;

        if let Some(old) = self.current {
            let throttle = rt::ServerView::<N>::with(&mut self.threads, |view| {
                self.rt_scheduler.put(old, dt, view)
            });

            if let Some(throttle) = throttle {
                if throttle <= now {
                    rt::ServerView::<N>::with(&mut self.threads, |view| {
                        let _ = self.rt_scheduler.dequeue(old, view);
                        let _ = self.rt_scheduler.enqueue(old, now, view);
                    });
                } else {
                    // This ensures that sleep_until will not trigger a reschedule.
                    self.current = None;
                    let _ = self.sleep_until(Some(old), throttle, now);
                    self.current = Some(old);
                }
            } else {
                self.rr_scheduler.put(old, dt as u32);
            }
        }

        self.do_wakeups(now);
    }

    fn select_next(&mut self) -> (thread::UId, u32) {
        rt::ServerView::<N>::with(&mut self.threads, |view| self.rt_scheduler.pick(view))
            .or_else(|| self.rr_scheduler.pick(&mut self.threads))
            .unwrap_or((thread::IDLE_THREAD, 1000))
    }

    /// Picks the next thread to run and returns its context and task. This should only be called by sched_enter after land.
    fn do_sched(&mut self, now: u64) -> Option<(*mut c_void, &mut task::Task)> {
        // Sync the new state to the scheduler.
        self.sync_to_sched(now);

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

    /// Puts a thread to sleep until the specified timepoint. This will trigger a reschedule if the thread is currently running.
    ///
    /// `uid` - The UID of the thread to put to sleep, or None to put the current thread to sleep.
    /// `until` - The timepoint to sleep until, in ticks. This is an absolute time, not a relative time.
    /// `now` - The current timepoint, in ticks.
    ///
    /// Returns an error if there is no current thread, it is not enqueued, or if the specified timepoint is in the past.
    pub fn sleep_until(&mut self, uid: Option<thread::UId>, until: u64, now: u64) -> Result<()> {
        if until <= now {
            return Ok(());
        }
        let uid = match uid {
            Some(uid) => uid,
            None => self.current.ok_or(kerr!(EINVAL))?,
        };
        // Make the thread not runnable. Triggers a reschedule if the thread is currently running.
        // If it fails, it means the thread was not enqueued, which is fine.
        let _ = self.dequeue(uid);

        // Check if the thread is already sleeping.
        let already = match self.threads.get_mut(uid) {
            Some(thread) if thread.is_waiting() => true,
            Some(_) => false,
            None => return Err(kerr!(EINVAL)),
        };

        // If the thread already sleeps, remove it from the wakeup tree.
        if already {
            WaiterView::with(&mut self.threads, |view| self.wakeup.remove(uid, view))?;
        }

        // Put the thread to sleep until the specified timepoint.
        if let Some(thread) = self.threads.get_mut(uid) {
            thread.wait(until);
        } else {
            // This should not be possible. The thread was just checked to exist.
            bug!("failed to set thread {} to waiting. Does not exist.", uid);
        }

        // Insert the thread into the wakeup tree.
        let res = WaiterView::with(&mut self.threads, |view| self.wakeup.insert(uid, view));

        if res.is_err() {
            // This should not be possible. The thread was just checked to exist.
            bug!("failed to insert thread {} into wakeup tree.", uid);
        }
        Ok(())
    }

    /// `kick` lookup by raw `UId::as_usize()`. Synthetic `tid` is a placeholder.
    pub fn kick_by_uid(&mut self, uid: usize) -> Result<()> {
        let lookup_uid = thread::UId::new(uid, thread::Id::new(0, crate::sched::task::UId::new(0)));
        self.kick(lookup_uid)
    }

    pub fn current_uid(&self) -> Option<usize> {
        self.current.map(|uid| uid.as_usize())
    }

    /// If the thread is currently sleeping, this will trigger a wakeup on the immediately following reschedule.
    ///
    /// Returns an error if the thread does not exist, or if the thread is not currently sleeping.
    pub fn kick(&mut self, uid: thread::UId) -> Result<()> {
        let now = time::tick();
        let res = WaiterView::with(&mut self.threads, |view| self.wakeup.remove(uid, view));

        if let Some(thread) = self.threads.get_mut(uid) {
            thread.resume();
        } else {
            return Err(kerr!(EINVAL)); // Thread does not exist.
        }

        if res.is_ok() {
            self.enqueue(now, uid)?;
        }
        Ok(())
    }

    /// This will make the thread not runnable, but it will not remove it from other lists.
    /// If the thread is currently running, reschedule will be triggered.
    ///
    /// Returns an error if the thread does not exist, or if the thread is not currently enqueued in any scheduler.
    pub fn dequeue(&mut self, uid: thread::UId) -> Result<()> {
        rt::ServerView::<N>::with(&mut self.threads, |view| {
            self.rt_scheduler.dequeue(uid, view)
        })
        .or_else(|_| self.rr_scheduler.dequeue(uid, &mut self.threads))?;

        if Some(uid) == self.current {
            reschedule();
        }
        Ok(())
    }

    pub fn create_task(&mut self, attrs: task::Attributes) -> Result<task::UId> {
        let task_id = self.tasks.insert_with(|idx| {
            let task = task::Task::new(task::UId::new(idx), attrs);
            task.map(|t| (task::UId::new(idx), t))
        })?;

        #[cfg(any(feature = "metrics", metrics))]
        if let Some(task) = self.tasks.get(task_id) {
            crate::metrics::store::write_task_heap(
                task_id.as_usize(),
                task.allocator_metrics().into(),
            );
        }

        Ok(task_id)
    }

    /// Dequeues all threads of the task and removes the task. If the current thread belongs to the task, reschedule will be triggered.
    ///
    /// If the task does not exist, an error will be returned.
    pub fn kill_by_task(&mut self, uid: task::UId) -> Result<()> {
        let task = self.tasks.get_mut(uid).ok_or(kerr!(EINVAL))?;

        while let Some(id) = task.threads().head() {
            kill!(self, id)?;

            if task.threads_mut().remove(id, &mut self.threads).is_err() {
                // This should not be possible. The thread ID is from the thread list of the task, so it must exist.
                bug!("failed to remove thread {} from task {}.", id, uid);
            }

            if self.threads.remove(&id).is_none() {
                // This should not be possible. The thread ID is from the thread list of the task, so it must exist.
                bug!("failed to remove thread {} from thread list.", id);
            }

            #[cfg(any(feature = "metrics", metrics))]
            crate::metrics::store::clear_thread_stack(id.as_usize());

            if Some(id) == self.current {
                self.current = None;
                reschedule();
            }
        }

        self.tasks.remove(&uid).ok_or(kerr!(EINVAL))?;

        #[cfg(any(feature = "metrics", metrics))]
        crate::metrics::store::clear_task_heap(uid.as_usize());

        Ok(())
    }

    pub fn create_thread(
        &mut self,
        task: Option<task::UId>,
        attrs: &thread::Attributes,
    ) -> Result<thread::UId> {
        let task = match task {
            Some(t) => t,
            None => self.current.ok_or(kerr!(EINVAL))?.owner(),
        };
        let task = self.tasks.get_mut(task).ok_or(kerr!(EINVAL))?;

        let uid = self
            .threads
            .insert_with(|idx| {
                let uid = task.allocate_tid().get_uid(idx);
                let stack = task.allocate_stack(attrs)?;
                let thread = thread::Thread::new(uid, stack, attrs.attrs);
                Ok((uid, thread))
            })
            .and_then(|k| {
                task.register_thread(k, &mut self.threads)?;
                Ok(k)
            })?;

        #[cfg(any(feature = "metrics", metrics))]
        if let Some(thread) = self.threads.get(uid) {
            crate::metrics::store::write_thread_stack(
                uid.as_usize(),
                thread.stack_metrics().into(),
            );
        }

        Ok(uid)
    }

    /// Dequeues a thread and removes it from its corresponding task. If the thread is currently running, reschedule will be triggered.
    ///
    /// `uid` - The UID of the thread to kill, or None to kill the current thread.
    ///
    /// If the thread does not exist, or if `uid` is None and there is no current thread, an error will be returned.
    pub fn kill_by_thread(&mut self, uid: Option<thread::UId>) -> Result<()> {
        let uid = match uid {
            Some(uid) => uid,
            None => self.current.ok_or(kerr!(EINVAL))?,
        };
        kill!(self, uid)?;

        self.tasks
            .get_mut(uid.tid().owner())
            .ok_or(kerr!(EINVAL))?
            .threads_mut()
            .remove(uid, &mut self.threads)?;

        self.threads.remove(&uid).ok_or(kerr!(EINVAL))?;

        #[cfg(any(feature = "metrics", metrics))]
        crate::metrics::store::clear_thread_stack(uid.as_usize());

        if Some(uid) == self.current {
            self.current = None;
            reschedule();
        }
        Ok(())
    }

    /// Updates the lock-free mirror for the currently scheduled thread and its task.
    /// Called on every reschedule; only the thread that just ran needs updating.
    #[cfg(any(feature = "metrics", metrics))]
    fn mirror_stats(&self) {
        use crate::metrics::store;

        store::write_global_heap(crate::mem::global_metrics().into());

        if let Some(uid) = self.current {
            if let Some(thread) = self.threads.get(uid) {
                store::write_thread_stack(uid.as_usize(), thread.stack_metrics().into());

                let task_id = thread.task_id();
                if let Some(task) = self.tasks.get(task_id) {
                    store::write_task_heap(task_id.as_usize(), task.allocator_metrics().into());
                }
            }
        }
    }
}

/// This function provides safe access to the global scheduler.
/// It disables interrupts and locks the scheduler. Use with caution!
pub fn with<T, F: FnOnce(&mut GlobalScheduler) -> T>(f: F) -> T {
    // Must mask *all* interrupts: the ISR-callable `kick_thread` re-enters
    // `with`, so any priority-selective mask would deadlock if an ISR
    // preempted a holder.
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

/// Wake a thread by raw `uid`. C-FFI so ISR-context callers can use it
/// without going through the syscall path. Errors are swallowed:
/// not-yet-sleeping is normal.
#[unsafe(no_mangle)]
pub extern "C" fn kick_thread(uid: u32) {
    with(|sched| {
        let _ = sched.kick_by_uid(uid as usize);
    });
    reschedule();
}

/// This will be called by the architecture-specific code to enter the scheduler. It will land the current thread, pick the next thread to run, and return its context and task.
#[unsafe(no_mangle)]
pub extern "C" fn sched_enter(mut ctx: *mut c_void) -> *mut c_void {
    with(|sched| {
        let old = sched.current.map(|c| c.owner());
        sched.land(ctx);

        // Mirror stats while self.current still points to the outgoing thread —
        // its stack context was just saved by land() and its task reflects any
        // allocations made since the last reschedule.
        #[cfg(any(feature = "metrics", metrics))]
        sched.mirror_stats();

        if let Some((new, task)) = sched.do_sched(time::tick()) {
            if old != Some(task.id) {
                dispch::prepare(task);
            }
            ctx = new;
        }

        ctx
    })
}

extern "C" fn thread_finalizer() -> ! {
    with(|sched| {
        if sched.kill_by_thread(None).is_err() {
            bug!("failed to terminate returned thread.");
        }
    });
    loop {
        hal::asm::nop!();
    }
}
