
pub fn sleep(until: u64) {
   hal::asm::syscall!(1, (until >> 32) as u32, until as u32);
}

pub fn sleep_for(duration: u64) {
   hal::asm::syscall!(2, (duration >> 32) as u32, duration as u32);
}