use crate::hal;

use hal::excep::ExcepStackFrame;

#[unsafe(no_mangle)]
pub extern "C" fn handle_hard_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::excep::ExcepBacktrace::new(ExcepStackFrame::new(stack), initial_fp);

    // TODO extract other diagnostic information
    panic!("A hard fault has been triggered.\n{}", backtrace);
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_mem_manage_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::excep::ExcepBacktrace::new(ExcepStackFrame::new(stack), initial_fp);

    // TODO extract other diagnostic information
    panic!(
        "A memory management fault has been triggered.\n{}",
        backtrace
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_bus_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::excep::ExcepBacktrace::new(ExcepStackFrame::new(stack), initial_fp);

    // TODO extract other diagnostic information
    panic!("A bus fault has been triggered.\n{}", backtrace);
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_usage_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::excep::ExcepBacktrace::new(ExcepStackFrame::new(stack), initial_fp);

    // TODO extract other diagnostic information
    panic!("A usage fault has been triggered.\n{}", backtrace);
}
