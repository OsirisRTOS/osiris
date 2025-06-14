use core::str;

use crate::kprintln;
use macros::syscall_handler;

#[syscall_handler(num = 0)]
fn syscall_print(fd: usize, buf: *const u8, len: usize) -> usize {
    if fd == 0 {
        let bytes = unsafe { core::slice::from_raw_parts(buf, len) };

        if let Ok(s) = str::from_utf8(bytes) {
            // If the bytes are valid UTF-8, print them directly
            kprintln!("[uspace] {}", s);
            return 0;
        }
    }

    1
}
