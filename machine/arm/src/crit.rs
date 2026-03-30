use critical_section::RawRestoreState;

struct CriticalSection;
critical_section::set_impl!(CriticalSection);

unsafe impl critical_section::Impl for CriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        crate::asm::disable_irq_save()
    }

    unsafe fn release(token: RawRestoreState) {
        crate::asm::enable_irq_restr(token);
    }
}