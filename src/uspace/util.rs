use crate::sched;

#[macro_export]
macro_rules! DECLARE_TASK {
    (mem_size: $mem_size:expr, stack_size: $stack_size:expr, name: $name:ident) => {
        const TASK_ ## $name ## _MEM_SIZE: usize = $mem_size;
        const TASK_ ## $name ## _STACK_SIZE: usize = $stack_size;
    };
}

pub extern "C" fn thread_finalizer() {
    let _ = hal::hprintln!("debug: thread finalizer called.");

    sched::reschedule();
}