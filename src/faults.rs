use hal::Machinelike;

#[unsafe(no_mangle)]
pub extern "C" fn handle_hard_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::Machine::backtrace(initial_fp, stack);
    // TODO extract other diagnostic information
    panic!("A hard fault has been triggered.\n{backtrace}");
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_mem_manage_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::Machine::backtrace(initial_fp, stack);
    let fault_status = hal::Machine::get_fault_status(hal::Fault::MemManage);

    // TODO extract other diagnostic information
    panic!("A memory management fault has been triggered.\n{backtrace}{fault_status}");
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_bus_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::Machine::backtrace(initial_fp, stack);
    let fault_status = hal::Machine::get_fault_status(hal::Fault::Bus);

    // TODO extract other diagnostic information
    panic!("A bus fault has been triggered.\n{backtrace}{fault_status}");
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_usage_fault(stack: *const usize, initial_fp: *const usize) -> ! {
    let backtrace = hal::Machine::backtrace(initial_fp, stack);
    let fault_status = hal::Machine::get_fault_status(hal::Fault::Usage);

    // TODO extract other diagnostic information
    panic!("A usage fault has been triggered.\n{backtrace}{fault_status}");
}
