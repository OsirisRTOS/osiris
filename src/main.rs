#![cfg_attr(freestanding, no_std, no_main)]

#[cfg(freestanding)]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    hal::asm::startup_trampoline!();
}

/// The panic handler.
#[cfg(freestanding)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use hal::Machinelike;

    osiris::kprintln!("**************************** PANIC ****************************");
    osiris::kprintln!("");
    osiris::kprintln!("Message: {}", info.message());

    if let Some(location) = info.location() {
        osiris::kprintln!("Location: {}:{}", location.file(), location.line());
    }

    osiris::kprintln!("**************************** PANIC ****************************");

    hal::Machine::panic_handler(info);
}

#[cfg(not(freestanding))]
fn main() {
    println!("Hello from Osiris!");
}
