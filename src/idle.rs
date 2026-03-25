use crate::sched;

extern "C" fn entry() {
    loop {
        hal::asm::wfi!();
    }
}

pub fn init() {
    let attrs = sched::thread::Attributes {
        entry: entry,
        fin: None,
    };
    if let Err(e) = sched::create_thread(sched::task::KERNEL_TASK, &attrs) {
        panic!("[Idle] Error: failed to create idle thread. Error: {e:?}");
    }
}