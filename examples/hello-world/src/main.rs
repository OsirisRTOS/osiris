#![no_std]
#![no_main]

use osiris::app_main;

extern "C" fn second_thread() {
    osiris::uprintln!("Hello from the second thread!");

    let mut tick = 0;
    for i in 0..5 {
        osiris::uprintln!("Second thread tick: {}", tick);
        tick += 1;
        osiris::uapi::sched::sleep_for(1500);
    }

    osiris::uapi::sched::exit(0);
    osiris::uprintln!("This will never be printed.");
}

extern "C" fn generator_thread() {

    let mut cnt = 0;
    loop {
        osiris::uapi::sched::yield_thread();
        osiris::uprintln!("Number: {}", cnt);
        cnt += 1;
    }
}

#[app_main]
fn main() {
    osiris::uprintln!("Hello World!");
    let mut tick = 0;
    osiris::uapi::sched::spawn_thread(second_thread);
    osiris::uapi::sched::spawn_thread(generator_thread);
    loop {
        osiris::uprintln!("Tick: {}", tick);
        tick += 1;
        osiris::uapi::sched::sleep_for(1000);
    }
}
