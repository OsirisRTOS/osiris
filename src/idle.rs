use crate::hal;
use crate::sched;

extern "C" fn entry() {
    loop {
        hal::asm::nop!();
    }
}

pub fn init() {
    let attrs = sched::thread::Attributes {
        entry,
        fin: None,
        attrs: None,
    };
    sched::with(|sched| {
        if let Err(e) = sched.create_thread(Some(sched::task::KERNEL_TASK), &attrs) {
            panic!("failed to create idle thread. Error: {}", e);
        }
    });
}
