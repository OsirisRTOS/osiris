use crate::modules::KernelModule;
use crate::utils::KernelError;

#[derive(Default)]
pub(super) struct SampleModule {
    value: u32
}

impl KernelModule for SampleModule {
    fn init(&mut self) -> Result<(), KernelError> {
        Ok(())
    }

    fn exit(&mut self) -> Result<(), KernelError> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "sample Module"
    }
}
impl SampleModule {
    pub(crate) const fn new() -> Self {
        SampleModule { value: 0 }
    }
}