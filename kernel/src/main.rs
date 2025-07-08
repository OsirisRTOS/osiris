#![cfg_attr(all(not(test), not(doctest), not(doc), not(kani)), no_std, no_main)]

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub extern "C" fn _start() -> ! {
    hal::asm::startup_trampoline!();
}

/// The panic handler.
#[cfg(all(not(test), not(doctest), not(doc), target_os = "none"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    kernel::kprintln!("**************************** PANIC ****************************");
    kernel::kprintln!("");
    kernel::kprintln!("Message: {}", info.message());

    if let Some(location) = info.location() {
        kernel::kprintln!("Location: {}:{}", location.file(), location.line());
    }

    kernel::kprintln!("**************************** PANIC ****************************");

    hal::panic::panic_handler(info);
}