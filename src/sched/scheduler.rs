use hal::common::{sched::{CtxPtr, ThreadContext, ThreadDesc}, sync::SpinLocked};

use crate::mem::{self, alloc::AllocError, array::IndexMap};

use super::task::{Task, TaskDesc, TaskId, TaskMemory};

static SCHEDULER: SpinLocked<Scheduler> = SpinLocked::new(Scheduler::new());

struct Scheduler {
    current: TaskId,
    tasks: IndexMap<Task, 4>
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            current: TaskId::Kernel,
            tasks: IndexMap::new(),
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

    pub fn reschedule(&mut self) -> Option<CtxPtr> {
        let mut id = self.current;
        let mut ctx = None;
    
        if let Some(next) = self.tasks.next(id.into()) {
            if let Some(task) = self.tasks.get(next) {
                if let Some(new_ctx) = task.get_active_ctx() {
                    id = TaskId::from(next);
                    ctx = Some((*new_ctx).into());
                }
            }
        }
    
        self.current = id;
        ctx
    }
}

pub fn reschedule() -> Option<CtxPtr> {
    SCHEDULER.lock().reschedule()
}

pub fn create_task(desc: TaskDesc, init_desc: ThreadDesc) -> Result<TaskId, AllocError> {
    SCHEDULER.lock().create_task(desc, init_desc)
}

pub fn add_task(task: Task) -> Result<TaskId, AllocError> {
    SCHEDULER.lock().add_task(task)
}

/// cbindgen:ignore
/// cbindgen:no-export
#[no_mangle]
pub extern "C" fn sched_enter(ctx: CtxPtr) -> CtxPtr {
    {
        let mut scheduler = SCHEDULER.lock();
        let id = scheduler.current;

        if let Some(task) = scheduler.tasks.get_mut(id.into()) {
            task.save_context(task.get_active_thread(), ctx.into());
        }
    }
    
    reschedule().unwrap_or(ctx)
}


