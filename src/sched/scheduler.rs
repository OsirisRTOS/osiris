use hal::common::{sched::{CtxPtr, ThreadContext, ThreadDesc}, sync::SpinLocked};

use crate::mem::{self, alloc::AllocError, array::IndexMap};

use super::task::{Task, TaskDesc, TaskId, TaskMemory};

pub static SCHEDULER: SpinLocked<Scheduler> = SpinLocked::new(Scheduler::new(1));

pub struct Scheduler {
    current: Option<TaskId>,
    tasks: IndexMap<Task, 4>,
    time: usize,
    interval: usize,
}

impl Scheduler {
    pub const fn new(interval: usize) -> Self {
        Self {
            current: None,
            tasks: IndexMap::new(),
            time: 0,
            interval
        }
    }

    pub fn create_task(&mut self, desc: TaskDesc, init_desc: ThreadDesc) -> Result<TaskId, AllocError> {
        let size = mem::align_up(desc.mem_size) + mem::align_up(desc.stack_size);
        let memory = TaskMemory::new(size)?;
        
        let ctx = unsafe { ThreadContext::from_empty(memory.stack(), init_desc) };

        self.add_task(Task::new(memory, ctx))
    }

    pub fn add_task(&mut self, task: Task) -> Result<TaskId, AllocError> {
        let task_id = self.tasks.insert_next(task)?;

        if let Some(task) = self.tasks.get_mut(task_id) {
            task.id = task_id.into();
            return Ok(task_id.into());
        }

        Err(AllocError::OutOfMemory)
    }

    fn select_task(&mut self) -> Option<CtxPtr> {
        let mut id = self.current.map(|id| id.into());
        let mut ctx = None;

        if let Some(next) = self.tasks.next(id) {
            if let Some(task) = self.tasks.get(next) {
                if let Some(new_ctx) = task.get_active_ctx() {
                    id = Some(next);
                    ctx = Some((*new_ctx).into());
                }
            }
        }
    
        self.current = id.map(TaskId::from);
        ctx
    }

    fn tick(&mut self) -> bool {
        self.time += 1;

        if self.time >= self.interval {
            self.time = 0;
            return true;
        }

        false
    }
}

/// cbindgen:ignore
/// cbindgen:no-export
#[no_mangle]
pub extern "C" fn sched_enter(ctx: CtxPtr) -> CtxPtr {
    hal::hprintln!("Scheduler resched.").unwrap();

    {
        let mut scheduler = SCHEDULER.lock();

        if let Some(id) = scheduler.current {
            if let Some(task) = scheduler.tasks.get_mut(id.into()) {
                task.save_context(task.get_active_thread(), ctx.into());
            }
        }

        scheduler.select_task().unwrap_or(ctx)
    }
}

/// cbindgen:ignore
/// cbindgen:no-export
#[no_mangle]
pub extern "C" fn systick() {
    let resched = {
        let mut scheduler = SCHEDULER.lock();
        scheduler.tick()
    };

    if resched {
        hal::common::sched::reschedule();
    }
}



