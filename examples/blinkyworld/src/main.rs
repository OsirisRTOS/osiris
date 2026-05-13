#![no_std]
#![no_main]

use core::ffi::c_void;

use osiris::app_main;
use osiris::uapi::led::Led;
use osiris::uapi::print;
use osiris::uapi::sched::{RtAttrs, sleep, sleep_for, spawn_thread};

struct BlinkyLed {
    alias: &'static str,
    half_period_ms: u32,
}

static GREEN: BlinkyLed = BlinkyLed {
    alias: "led0",
    half_period_ms: 100,
};
static BLUE: BlinkyLed = BlinkyLed {
    alias: "led1",
    half_period_ms: 200,
};
static RED: BlinkyLed = BlinkyLed {
    alias: "led2",
    half_period_ms: 500,
};

extern "C" fn run(ctx: *mut c_void) {
    // SAFETY: every caller hands us a pointer to a BlinkyLed, which live for the entire program.
    let blinky_led = unsafe { &*(ctx as *const BlinkyLed) };

    let led = match Led::open_by_alias(blinky_led.alias) {
        Ok(l) => l,
        Err(e) => {
            print::print(format_args!(
                "blinkyworld: Led::open({}) failed: {:?}\n",
                blinky_led.alias, e
            ));
            return;
        }
    };

    let half = blinky_led.half_period_ms as u64;
    loop {
        let _ = led.on();
        sleep_for(half);
        let _ = led.off();
        sleep_for(half);
    }
}

#[app_main]
fn main() {
    print::print(format_args!(
        "blinkyworld: spawning blinkers green=100ms blue=200ms red=500ms\n"
    ));

    let attrs = Some(RtAttrs {
        budget: 100,
        deadline: 100,
        period: 100,
    });

    for BlinkyLed in [&GREEN, &BLUE, &RED] {
        let ctx = BlinkyLed as *const BlinkyLed as *mut c_void;
        if spawn_thread(run, ctx, attrs) < 0 {
            print::print(format_args!(
                "blinkyworld: spawn_thread for {} failed\n",
                BlinkyLed.alias
            ));
        }
    }
}
