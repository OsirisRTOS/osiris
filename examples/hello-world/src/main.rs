#![no_std]
#![no_main]

use osiris::app_main;

extern "C" fn second_thread() {
    osiris::uprintln!("Hello from the second thread!");

    let mut tick = 0;
    loop {
        osiris::uprintln!("Second thread tick: {}", tick);
        tick += 1;
        osiris::uapi::sched::sleep_for(1500);
    }
}

#[app_main]
fn main() {
    osiris::uprintln!("Hello World!");
    let mut tick = 0;
    osiris::uapi::sched::spawn_thread(second_thread);

    loop {
        osiris::uprintln!("Tick: {}", tick);
        tick += 1;
        osiris::uapi::sched::sleep_for(1000);
    }
}
