mod sample_module;

use crate::modules::kernelmods::sample_module::SampleModule;
use crate::modules::KernelModule;
use crate::sync::spinlock::SpinLock;
use crate::utils::KernelError;

//Lock to guarantee race condition free access
static LOCK: SpinLock = SpinLock::new();

//Generate per Module
static mut MODULE_A: SampleModule = SampleModule::new();
static mut MODULE_B: SampleModule = SampleModule::new();



pub(super) fn init_modules() -> Result<(), KernelError> {
    //SAFETY: All kernel modules are private to this generated file and are secured using a common lock, therefor no race conditions can appear
    unsafe {
        LOCK.lock();
        let res = MODULE_A.init();
        if res.is_err() {LOCK.unlock();return res;}
        let res = MODULE_B.init();
        if res.is_err() {LOCK.unlock();return res;}
        LOCK.unlock();
    }
    Ok(())
}

pub(super) fn exit_modules() -> Result<(), KernelError> {
    //SAFETY: All kernel modules are private to this generated file and are secured using a common lock, therefor no race conditions can appear
    unsafe {
        LOCK.lock();
        let res = MODULE_A.exit();
        if res.is_err() {LOCK.unlock();return res;}
        let res = MODULE_B.exit();
        if res.is_err() {LOCK.unlock();return res;}
        LOCK.unlock();
    }
    Ok(())
}