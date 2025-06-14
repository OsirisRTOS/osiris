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

    let mut syms_start = &raw const __syms_area_start as *const usize as usize;

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

    while current < syms_start + size {
        let entry = unsafe { &*(current as *const SymtabEntry) };

        // Calculate the distance from the address to the symbol value.
        let distance = if addr > entry.value {
            addr - entry.value
        } else {
            entry.value - addr
        };

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
