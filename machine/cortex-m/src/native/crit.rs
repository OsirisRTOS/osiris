use critical_section::RawRestoreState;

struct CriticalSection;
critical_section::set_impl!(CriticalSection);

unsafe impl critical_section::Impl for CriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        super::asm::disable_irq_save()
    }

    unsafe fn release(token: RawRestoreState) {
        super::asm::enable_irq_restr(token);
    }
}
