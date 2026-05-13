#![no_std]
#![no_main]

use osiris::app_main;
use osiris::uapi::{key::Key, led::Led, print};

/// Drive the three Nucleo-L4R5ZI LEDs (green/blue/red on `led0`/`led1`/
/// `led2`) as a 3-bit binary counter that advances on every USER button
/// press (`sw0`). Releases are ignored.
#[app_main]
fn main() {
    print::print(format_args!("gpio-button: opening led0..led2 + sw0\n"));

    let leds = match (
        Led::open_by_alias("led0"),
        Led::open_by_alias("led1"),
        Led::open_by_alias("led2"),
    ) {
        (Ok(g), Ok(b), Ok(r)) => [g, b, r],
        (g, b, r) => {
            print::print(format_args!(
                "Led::open failed: led0={:?} led1={:?} led2={:?}\n",
                g.err(),
                b.err(),
                r.err()
            ));
            return;
        }
    };

    let btn = match Key::open_by_alias("sw0") {
        Ok(k) => k,
        Err(e) => {
            print::print(format_args!("Key::open_by_alias(sw0) failed: {:?}\n", e));
            return;
        }
    };

    let mut count: u8 = 0;
    apply(&leds, count);

    loop {
        match btn.wait() {
            Ok(evt) if evt.pressed => {
                count = count.wrapping_add(1) & 0b111;
                apply(&leds, count);
            }
            Ok(_) => {
                // Ignore button release
            }
            Err(e) => {
                print::print(format_args!("btn.wait error: {:?}\n", e));
                return;
            }
        }
    }
}

fn apply(leds: &[Led; 3], count: u8) {
    for (i, led) in leds.iter().enumerate() {
        led.set(count & (1 << i) != 0).expect("led.set failed");
    }
}
