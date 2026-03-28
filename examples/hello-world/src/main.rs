#![no_std]
#![no_main]

use osiris::app_main;

#[app_main]
fn main() {
    osiris::uprintln!("Hello World!");
    let mut tick = 0;

    loop {
        osiris::uprintln!("Tick: {}", tick);
        tick += 1;
        osiris::uapi::sched::sleep_for(1000);
    }
}
