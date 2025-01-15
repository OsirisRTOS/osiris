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
        let alloc = mem::malloc(size, align_of::<u128>()).ok_or(AllocError::OutOfMemory)?;
        let memory = TaskMemory::new(alloc, size);
        
        let ctx = unsafe { ThreadContext::from_empty(memory.stack(), init_desc) };
        let task_id = self.tasks.insert_next(Task::new(memory, ctx))?;

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

pub fn reschedule() -> Option<CtxPtr> {
    
    SCHEDULER.lock().reschedule()
}

pub fn create_task(desc: TaskDesc, init_desc: ThreadDesc) -> Result<TaskId, AllocError> {
    SCHEDULER.lock().create_task(desc, init_desc)
}


