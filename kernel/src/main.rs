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

    kernel::kprintln!("**************************** PANIC ****************************");
    kernel::kprintln!("");
    kernel::kprintln!("Message: {}", info.message());

    if let Some(location) = info.location() {
        kernel::kprintln!("Location: {}:{}", location.file(), location.line());
    }

    kernel::kprintln!("**************************** PANIC ****************************");

    hal::Machine::panic_handler(info);
}

#[cfg(not(freestanding))]
fn main() {
    println!("Hello from Osiris!");
}
