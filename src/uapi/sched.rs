use hal::stack::EntryFn;


pub fn sleep(until: u64) -> isize {
     hal::asm::syscall!(1, (until >> 32) as u32, until as u32)
}
  
pub fn sleep_for(duration: u64) -> isize {
   hal::asm::syscall!(2, (duration >> 32) as u32, duration as u32)
}

pub fn spawn_thread(func_ptr: EntryFn) -> isize {
    hal::asm::syscall!(3, func_ptr as u32)
}