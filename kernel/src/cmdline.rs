

#[repr(C)]
pub struct InitDescriptor {
    /// Pointer to the start of the binary of the init program.
    pub begin: *const usize,
    /// Length of the binary of the init program.
    pub len: usize,
    pub entry_offset: usize,
}

#[repr(C)]
pub struct Args {
    pub init: InitDescriptor,
}