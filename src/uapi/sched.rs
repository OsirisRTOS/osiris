use hal::stack::EntryFn;

pub fn sleep(_until: u64) -> isize {
    hal::asm::syscall!(1, (_until >> 32) as u32, _until as u32)
}

pub fn sleep_for(_duration: u64) -> isize {
    hal::asm::syscall!(2, (_duration >> 32) as u32, _duration as u32)
}

pub fn yield_thread() -> isize {
    let _until = u64::MAX;
    hal::asm::syscall!(1, (_until >> 32) as u32, _until as u32)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RtAttrs {
    pub deadline: u64,
    pub period: u32,
    pub budget: u32,
}

pub fn spawn_thread(_func_ptr: EntryFn, attrs: Option<RtAttrs>) -> isize {
    let _attr_ptr = if let Some(attrs) = attrs {
        &attrs as *const RtAttrs as usize
    } else {
        0
    };
    hal::asm::syscall!(3, _func_ptr as u32, _attr_ptr)
}

pub fn exit(_code: usize) -> ! {
    hal::asm::syscall!(4, _code as u32);
    loop {
        hal::asm::nop!();
    }
}
