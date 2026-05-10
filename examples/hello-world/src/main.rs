#![no_std]
#![no_main]

use core::ffi::c_void;
use core::ptr::null_mut;

use osiris::app_main;

extern "C" fn second_thread(_ctx: *mut c_void) {
    let mut time = osiris::uapi::time::tick();
    let mut cnt = 0;
    loop {
        time += 100;
        osiris::uprintln!("Number: {}", cnt);
        cnt += 1;
        osiris::uapi::sched::sleep(time);
    }
}

#[app_main]
fn main() {
    osiris::uprintln!("Hello World!");
    let mut tick = 0;
    let attrs = osiris::uapi::sched::RtAttrs {
        deadline: 100,
        period: 100,
        budget: 100,
    };

    osiris::uapi::sched::spawn_thread(second_thread, null_mut(), Some(attrs));
    loop {
        osiris::uprintln!("Tick: {}", tick);
        tick += 1;
        osiris::uapi::sched::sleep_for(1000);
    }
}
