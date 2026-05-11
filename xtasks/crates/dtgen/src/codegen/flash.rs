//! Flash partition registry codegen.

use super::*;

#[derive(Clone)]
struct Partition {
    node: usize,
    flash_node: usize,
    label: String,
    compatible: String,
    offset: usize,
    len: usize,
    read_only: bool,
}

/// Returns `(node_idx, base, size)` of the first `flash@*` node with a
/// valid `reg`, if any. All partitions are assumed to live in this flash;
/// multi-flash systems would need an extension here.
fn find_primary_flash(dt: &DeviceTree) -> Option<(usize, usize, usize)> {
    for (idx, n) in dt.nodes.iter().enumerate() {
        if !n.name.starts_with("flash@") {
            continue;
        }
        let Some((base, size)) = n.reg else {
            continue;
        };
        let (Ok(base), Ok(size)) = (usize::try_from(base), usize::try_from(size)) else {
            continue;
        };
        return Some((idx, base, size));
    }
    None
}

fn collect(dt: &DeviceTree, primary_idx: Option<usize>) -> Vec<Partition> {
    let mut out: Vec<Partition> = Vec::new();

    for (flash_idx, flash_node) in dt.nodes.iter().enumerate() {
        if !flash_node.name.starts_with("flash@") {
            continue;
        }
        let is_primary = primary_idx == Some(flash_idx);

        // Find a child `partitions` node whose compatible includes "fixed-partitions".
        for &part_block_idx in &flash_node.children {
            let part_block = &dt.nodes[part_block_idx];
            if !part_block
                .compatible
                .iter()
                .any(|c| c == "fixed-partitions")
            {
                continue;
            }

            for &part_idx in &part_block.children {
                let part = &dt.nodes[part_idx];
                if !part.name.starts_with("partition@") {
                    continue;
                }

                if !is_primary {
                    eprintln!(
                        "cargo::warning=flash partition {} sits under non-primary flash node {}; skipping (multi-flash systems not yet supported)",
                        part.name, flash_node.name
                    );
                    continue;
                }

                let Some((offset_u64, len_u64)) = part.reg else {
                    eprintln!(
                        "cargo::warning=flash partition node {} missing reg property; skipping",
                        part.name
                    );
                    continue;
                };
                let (Ok(offset), Ok(len)) =
                    (usize::try_from(offset_u64), usize::try_from(len_u64))
                else {
                    eprintln!(
                        "cargo::warning=flash partition {} reg out of usize range; skipping",
                        part.name
                    );
                    continue;
                };

                let label = match part.extra.get("label") {
                    Some(PropValue::Str(s)) => s.clone(),
                    _ => {
                        eprintln!(
                            "cargo::warning=flash partition {} missing label; skipping",
                            part.name
                        );
                        continue;
                    }
                };

                let compatible = part
                    .compatible
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "fixed-partitions".to_string());

                let read_only = part.extra.contains_key("read-only");

                out.push(Partition {
                    node: part_idx,
                    flash_node: flash_idx,
                    label,
                    compatible,
                    offset,
                    len,
                    read_only,
                });
            }
        }
    }

    out
}

/// True iff any `partition@*` node exists anywhere in the DT.
fn any_partition_in_dt(dt: &DeviceTree) -> bool {
    dt.nodes.iter().any(|n| n.name.starts_with("partition@"))
}

pub fn emit_registry(dt: &DeviceTree) -> TokenStream {
    let primary = find_primary_flash(dt);
    let (flash_base, flash_total_size) = primary
        .map(|(_, base, size)| (base, size))
        .unwrap_or((0, 0));
    let parts = collect(dt, primary.map(|(idx, _, _)| idx));

    if primary.is_none() {
        if any_partition_in_dt(dt) {
            eprintln!(
                "cargo::error=no `flash@*` node with a valid `reg` property in device tree, but partition nodes are present — FLASH_BASE would be 0 and partition addresses would be bogus"
            );
            std::process::exit(1);
        } else {
            eprintln!(
                "cargo::warning=no `flash@*` node found in device tree; FLASH_BASE/FLASH_TOTAL_SIZE emitted as 0"
            );
        }
    }

    for p in &parts {
        let offset_end = p.offset.checked_add(p.len);
        let abs_end = flash_base
            .checked_add(p.offset)
            .and_then(|s| s.checked_add(p.len));
        let in_range = offset_end.map(|e| e <= flash_total_size).unwrap_or(false);
        if abs_end.is_none() || !in_range {
            eprintln!(
                "cargo::error=flash partition '{}' (offset={:#x}, len={:#x}) overflows or exceeds flash size {:#x}",
                p.label, p.offset, p.len, flash_total_size
            );
            std::process::exit(1);
        }
    }

    let entries = parts.iter().map(|p| {
        let node = p.node;
        let flash_node = p.flash_node;
        let label = p.label.as_str();
        let compatible = p.compatible.as_str();
        let offset = p.offset;
        let len = p.len;
        let read_only = p.read_only;
        quote! {
            FlashPartitionRegistryEntry {
                node: #node,
                flash_node: #flash_node,
                label: #label,
                compatible: #compatible,
                offset: #offset,
                len: #len,
                read_only: #read_only,
            },
        }
    });

    quote! {
        /// Absolute base address of the primary flash bank, sourced from
        /// the parent `flash@*` node's `reg` cell. Zero if no flash node
        /// is present in the device tree.
        pub const FLASH_BASE: usize = #flash_base;

        /// Total size of the primary flash, in bytes, from the same `reg`.
        /// Note that this is the static DT value; runtime queries (e.g.
        /// dual-bank vs single-bank page sizes) live in the chip HAL.
        pub const FLASH_TOTAL_SIZE: usize = #flash_total_size;

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct FlashPartitionRegistryEntry {
            pub node: usize,
            pub flash_node: usize,
            pub label: &'static str,
            pub compatible: &'static str,
            /// Offset within the flash, relative to `FLASH_BASE`.
            pub offset: usize,
            pub len: usize,
            pub read_only: bool,
        }

        pub const FLASH_PARTITION_REGISTRY: &[FlashPartitionRegistryEntry] = &[
            #(#entries)*
        ];
    }
}

pub fn emit_query_api() -> TokenStream {
    quote! {
        pub fn flash_partition_by_compatible(
            compatible: &str,
            ord: usize,
        ) -> Option<&'static FlashPartitionRegistryEntry> {
            let mut matches = 0usize;
            for p in FLASH_PARTITION_REGISTRY {
                if p.compatible == compatible {
                    if matches == ord {
                        return Some(p);
                    }
                    matches += 1;
                }
            }
            None
        }

        pub fn flash_partition_by_label(
            label: &str,
        ) -> Option<&'static FlashPartitionRegistryEntry> {
            FLASH_PARTITION_REGISTRY.iter().find(|p| p.label == label)
        }

        /// Find the partition that contains the given absolute flash
        /// address and return it together with the offset of that
        /// address from the partition's start.
        pub fn flash_partition_by_address(
            address: usize,
        ) -> Option<(&'static FlashPartitionRegistryEntry, usize)> {
            for p in FLASH_PARTITION_REGISTRY {
                let start = FLASH_BASE + p.offset;
                let end = start + p.len;
                if address >= start && address < end {
                    return Some((p, address - start));
                }
            }
            None
        }
    }
}
