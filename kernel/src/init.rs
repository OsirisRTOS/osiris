use crate::sched::task::TaskDesc;

use crate::utils::KernelError;
use hal::sched::ThreadDesc;
use crate::sched::task::Timing;
use crate::{sched, uspace};

unsafe extern "C" {
    fn init_main(argc: usize, argv: *const *const u8);
}

pub fn spawn_init_task() -> Result<(), KernelError> {
    // Create the init task.
    let init_thread = ThreadDesc {
        entry: init_main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    let init_task = TaskDesc {
        mem_size: 0,
        stack_size: 4096,
    };

    // TODO: These are dummy values for testing.
    let init_timing = Timing {
        period: 8,
        deadline: 8,
        exec_time: 2,
    };

    // Create the init task.
    sched::create_task(init_task, init_thread, init_timing)?;

    Ok(())
}