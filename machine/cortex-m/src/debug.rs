#[cfg(not(feature = "host"))]
unsafe extern "C" {
    static __syms_area_start: usize;
}

#[cfg(not(feature = "host"))]
#[repr(C)]
struct SymtabEntry {
    name: usize,  // Offset into the string table
    value: usize, // Address of the symbol
    size: usize,  // Size of the symbol
    info: u8,     // Type and binding information
    other: u8,    // Other information
    shndx: u16,   // Section index
}

#[cfg(not(feature = "host"))]
pub fn find_nearest_symbol(addr: usize) -> Option<&'static str> {
    use core::ffi::CStr;
    use core::ffi::c_char;

    let mut syms_start = &raw const __syms_area_start as usize;

    // Iterate through the symbol table to find the nearest symbol to the given address.
    let mut nearest_symbol: Option<&'static str> = None;
    let mut nearest_distance = usize::MAX;

    // The first 4 bytes in LE are the size of the symbol table.
    let size = unsafe { *(syms_start as *const usize) };
    syms_start += core::mem::size_of::<usize>(); // Move past the size field.

    // If we have no symbols, return None.
    if size == 0 {
        return None;
    }

    let mut current = syms_start;
    let strtab_start = syms_start + size;

    while current < syms_start + size - 1 {
        let entry = unsafe { &*(current as *const SymtabEntry) };

        // Calculate the distance from the address to the symbol value.
        let distance = addr.abs_diff(entry.value);

        // Check if this is the nearest symbol found so far.
        if distance < nearest_distance {
            nearest_distance = distance;

            let entry_name =
                unsafe { CStr::from_ptr((strtab_start + entry.name) as *const c_char) };
            nearest_symbol = entry_name.to_str().ok();
        }

        // Move to the next entry in the symbol table.
        current += core::mem::size_of::<SymtabEntry>();
    }

    nearest_symbol
}

#[cfg(feature = "host")]
pub fn find_nearest_symbol(_addr: usize) -> Option<&'static str> {
    // In host mode, we do not have a symbol table.
    None
}

#[cfg(all(not(feature = "host"), cm4))]
pub fn print_mem_manage_fault_status(
    f: &mut core::fmt::Formatter<'_>,
) -> Result<(), core::fmt::Error> {
    let cfsr = unsafe { core::ptr::read_volatile(0xE000ED28 as *const u32) };

    writeln!(f, "CFSR: 0x{cfsr:08x}")?;
    if cfsr & 0x1 != 0 {
        writeln!(f, "  IACCVIOL: Instruction access violation")?;
    }
    if cfsr & 0x2 != 0 {
        writeln!(f, "  DACCVIOL: Data access violation")?;
    }
    if cfsr & 0x8 != 0 {
        writeln!(f, "  MUNSTKERR: MemManage fault on unstacking")?;
    }
    if cfsr & 0x10 != 0 {
        writeln!(f, "  MSTKERR: MemManage fault on stacking")?;
    }
    if cfsr & 0x20 != 0 {
        writeln!(
            f,
            "  MLSPERR: MemManage fault during floating-point lazy state preservation"
        )?;
    }
    if cfsr & 0x80 != 0 {
        writeln!(
            f,
            "  MMARVALID: MemManage Fault Address Register (MMAR) is valid"
        )?;
    }

    Ok(())
}

#[cfg(all(not(feature = "host"), cm4))]
pub fn print_bus_fault_status(f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    let cfsr = unsafe { core::ptr::read_volatile(0xE000ED28 as *const u32) };

    writeln!(
        f,
        "---------------------------------------------------------------"
    )?;

    writeln!(f, "CFSR: 0x{cfsr:08x}")?;
    if cfsr & 0x100 != 0 {
        writeln!(f, "  IBUSERR: Instruction bus error")?;
    }
    if cfsr & 0x200 != 0 {
        writeln!(f, "  PRECISERR: Precise data bus error")?;
    }
    if cfsr & 0x400 != 0 {
        writeln!(f, "  IMPRECISERR: Imprecise data bus error")?;
    }
    if cfsr & 0x800 != 0 {
        writeln!(f, "  UNSTKERR: Bus fault on unstacking")?;
    }
    if cfsr & 0x1000 != 0 {
        writeln!(f, "  STKERR: Bus fault on stacking")?;
    }
    if cfsr & 0x2000 != 0 {
        writeln!(
            f,
            "  LSPERR: Bus fault during floating-point lazy state preservation"
        )?;
    }
    if cfsr & 0x8000 != 0 {
        writeln!(f, "  BFARVALID: Bus Fault Address Register (BFAR) is valid")?;
    }

    Ok(())
}

#[cfg(all(not(feature = "host"), cm4))]
pub fn print_usage_fault_status(f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    let cfsr = unsafe { core::ptr::read_volatile(0xE000ED28 as *const u32) };

    writeln!(
        f,
        "---------------------------------------------------------------"
    )?;

    writeln!(f, "CFSR: 0x{cfsr:08x}")?;
    if cfsr & 0x10000 != 0 {
        writeln!(f, "  UNDEFINSTR: Undefined instruction")?;
    }
    if cfsr & 0x20000 != 0 {
        writeln!(f, "  INVSTATE: Invalid state")?;
    }
    if cfsr & 0x40000 != 0 {
        writeln!(f, "  INVPC: Invalid PC load usage fault")?;
    }
    if cfsr & 0x80000 != 0 {
        writeln!(f, "  NOCP: No coprocessor")?;
    }
    if cfsr & 0x100000 != 0 {
        writeln!(f, "  UNALIGNED: Unaligned access")?;
    }
    if cfsr & 0x200000 != 0 {
        writeln!(f, "  DIVBYZERO: Divide by zero")?;
    }

    Ok(())
}

#[cfg(any(feature = "host", not(cm4)))]
pub fn print_mem_manage_fault_status(
    _f: &mut core::fmt::Formatter<'_>,
) -> Result<(), core::fmt::Error> {
    Ok(())
}

#[cfg(any(feature = "host", not(cm4)))]
pub fn print_bus_fault_status(_f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    Ok(())
}

#[cfg(any(feature = "host", not(cm4)))]
pub fn print_usage_fault_status(_f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
    Ok(())
}
