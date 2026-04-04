use crate::ir::DeviceTree;

fn format_region(name: &str, base: u64, size: u64) -> String {
    format!(
        "  {} : ORIGIN = 0x{:08x}, LENGTH = 0x{:08x}",
        name, base, size
    )
}

fn format_memory_section(regions: &[(&str, u64, u64)]) -> String {
    let regions = regions
        .iter()
        .map(|&(name, base, size)| format_region(name, base, size))
        .collect::<Vec<_>>()
        .join("\n");

    format!("MEMORY\n{{\n{regions}\n}}")
}

fn format_irq_provides(num: u32) -> String {
    format!("PROVIDE(__irq_{}_handler = default_handler);", num)
}

fn format_irq_section(num_irqs: u32) -> String {
    let provides = (0..num_irqs)
        .map(format_irq_provides)
        .collect::<Vec<_>>()
        .join("\n");

    format!("{provides}")
}

fn coalesce_regions<'a>(
    name: &'a str,
    regions: Vec<(&'a str, u64, u64)>,
) -> Result<Option<(&'a str, u64, u64)>, String> {
    regions
        .clone()
        .into_iter()
        .try_fold(None, |acc, (_, base, size)| {
            if let Some((_, acc_base, acc_size)) = acc {
                if base > acc_base + acc_size || acc_base > base + size {
                    return Err(format!("Regions are not contiguous. {regions:?}"));
                }

                let end = (base + size).max(acc_base + acc_size);
                let base = base.min(acc_base);
                let size = end - base;
                Ok(Some((name, base, size)))
            } else {
                Ok(Some((name, base, size)))
            }
        })
}

pub fn generate_ld(dt: &DeviceTree) -> Result<String, String> {
    // Generates a linker script prelude that defines the memory regions for the device tree.

    let mut ram: Vec<(&str, u64, u64)> = dt
        .nodes
        .iter()
        .filter(|n| n.name.starts_with("memory@") || n.name == "memory")
        .filter_map(|n| {
            let (base, size) = n.reg?;
            Some((n.name.as_str(), base, size))
        })
        .collect();
    ram.sort_by_key(|&(_, base, _)| base);

    let mut flash: Vec<(&str, u64, u64)> = dt
        .nodes
        .iter()
        .filter(|n| n.name.starts_with("flash@") || n.name == "flash")
        .filter_map(|n| {
            let (base, size) = n.reg?;
            Some((n.name.as_str(), base, size))
        })
        .collect();
    flash.sort_by_key(|&(_, base, _)| base);

    let flash = coalesce_regions("FLASH", flash)?;
    let ram = coalesce_regions("RAM", ram)?;

    let regions = flash.into_iter().chain(ram).collect::<Vec<_>>();

    if regions.is_empty() {
        return Err("No memory regions found in device tree".to_string());
    }

    // TODO: Derive the number of IRQs from the device tree.
    Ok(format!(
        "{}\n\n{}",
        format_memory_section(&regions),
        format_irq_section(240)
    ))
}
