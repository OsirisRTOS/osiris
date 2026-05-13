#![no_std]
#![no_main]

use core::ffi::c_void;

use osiris::app_main;
use osiris::uapi::led::Led;
use osiris::uapi::print;
use osiris::uapi::sched::{RtAttrs, sleep_for, spawn_thread};

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
    // SAFETY: every caller hands us a pointer to a static BlinkyLed,
    // which lives for the entire program.
    let cfg = unsafe { &*(ctx as *const BlinkyLed) };

    let led = Led::open_by_alias(cfg.alias).expect("Led::open_by_alias failed");
    let half = cfg.half_period_ms as u64;
    loop {
        led.on().expect("led.on failed");
        sleep_for(half);
        led.off().expect("led.off failed");
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

    for cfg in [&GREEN, &BLUE, &RED] {
        let ctx = cfg as *const BlinkyLed as *mut c_void;
        if spawn_thread(run, ctx, attrs) < 0 {
            print::print(format_args!(
                "blinkyworld: spawn_thread for {} failed\n",
                cfg.alias
            ));
        }
    }
}
