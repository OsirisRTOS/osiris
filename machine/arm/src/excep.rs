use core::fmt::Display;

#[repr(C)]
pub struct ExcepStackFrame {
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    psr: u32,
}

impl ExcepStackFrame {
    pub fn new(stack_ptr: *const usize) -> Self {
        unsafe {
            let frame = &*(stack_ptr as *const ExcepStackFrame);
            ExcepStackFrame {
                r0: frame.r0,
                r1: frame.r1,
                r2: frame.r2,
                r3: frame.r3,
                r12: frame.r12,
                lr: frame.lr,
                pc: frame.pc,
                psr: frame.psr,
            }
        }
    }
}

impl Display for ExcepStackFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "R0:  0x{:08x} R1:  0x{:08x} R2:  0x{:08x} R3:  0x{:08x}\n\
             R12: 0x{:08x} LR:  0x{:08x} PC:  0x{:08x} PSR: 0x{:08x}",
            self.r0, self.r1, self.r2, self.r3, self.r12, self.lr, self.pc, self.psr
        )
    }
}

const BACKTRACE_MAX_FRAMES: usize = 20;

#[repr(C)]
pub struct ExcepBacktrace {
    stack_frame: ExcepStackFrame,
    initial_fp: *const usize,
}

impl ExcepBacktrace {
    pub fn new(stack_frame: ExcepStackFrame, initial_fp: *const usize) -> Self {
        ExcepBacktrace {
            stack_frame,
            initial_fp,
        }
    }
}

impl Display for ExcepBacktrace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "---------------------------------------------------------------\n")?;
        write!(f, "{}\n", self.stack_frame)?;
        write!(f, "---------------------------------------------------------------\n")?;

        let mut fp = self.initial_fp;
        write!(f, "\nBacktrace:\n")?;


        if let Some(symbol) = crate::debug::find_nearest_symbol( self.stack_frame.pc as usize) {
            write!(f, "0:     {} (0x{:08x})\n", symbol, self.stack_frame.pc)?;
        } else {
            write!(f, "0:     0x{:08x}\n", self.stack_frame.pc)?;
        }

        for i in 1..BACKTRACE_MAX_FRAMES {
            // Read the return address from the stack.
            let ret_addr = unsafe { fp.add(1).read_volatile() };
            // Read the frame pointer from the current frame.
            let next_fp = unsafe { *fp };

            if ret_addr == 0 || ret_addr == 1 {
                break;
            }

            // Print the return address.

            if let Some(symbol) = crate::debug::find_nearest_symbol(ret_addr as usize) {
                write!(f, "{}:     {} (0x{:08x})\n", i, symbol, ret_addr)?;
            } else {
                write!(f, "{}:     0x{:08x}\n", i, ret_addr)?;
            }

            // If the next frame pointer is 0 or 1. (thumb mode adds +1 to the address)
            if next_fp == 0 || next_fp == 1 {
                break;
            }

            // Move to the next frame.
            fp = next_fp as *const usize;

            if i == BACKTRACE_MAX_FRAMES - 1 {
                write!(f, "{}:     ...\n", i)?;
            }
        }

        write!(f, "\n")
    }
}