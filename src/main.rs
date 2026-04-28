#![cfg_attr(freestanding, no_std, no_main)]

#[cfg(freestanding)]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn main() -> ! {
    osiris::hal::asm::startup_trampoline!();
}

#[unsafe(no_mangle)]
pub extern "C" fn app_main() -> ! {
    loop {}
}

/// The panic handler.
#[cfg(freestanding)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    osiris::panic(info);
}

#[cfg(not(freestanding))]
fn main() {
    println!("Hello from Osiris!");
}
