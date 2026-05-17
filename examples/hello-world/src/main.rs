#![no_std]
#![no_main]

use core::ffi::c_void;
use core::ptr::null_mut;

use osiris::app_main;

macro_rules! println {
    ($($arg:tt)*) => {{
        use osiris::uapi::*;
        // Print seconds and microseconds since boot.
        let (secs, frac) = to_secs(time::mono_now(), time::mono_freq() as u32, 6);
        print::print(format_args!("[{}.{:06}] ", secs, frac));
        print::print(format_args!($($arg)*));
        print::print(format_args!("\n"));
    }};
}

pub fn to_secs(cnt: u64, hz: u32, digits: u8) -> (u64, u64) {
    let secs = cnt / (hz as u64);
    let rem = cnt % (hz as u64);
    let frac = (rem * 10_u64.pow(digits as u32)) / (hz as u64);
    (secs, frac)
}

extern "C" fn second_thread(_ctx: *mut c_void) {
    let mut time = osiris::uapi::time::tick();
    let mut cnt = 0;
    loop {
        time += 100;
        println!("Number: {}", cnt);
        cnt += 1;
        osiris::uapi::sched::sleep(time);
    }
}

#[app_main]
fn main() {
    println!("Hello World!");
    let mut tick = 0;
    let attrs = osiris::uapi::sched::RtAttrs {
        deadline: 100,
        period: 100,
        budget: 100,
    };

    osiris::uapi::sched::spawn_thread(second_thread, null_mut(), Some(attrs));
    loop {
        println!("Tick: {}", tick);
        tick += 1;
        osiris::uapi::sched::sleep_for(1000);
    }
}
