use crate::ir::{DeviceTree, Node, PropValue};
use std::collections::HashMap;

// ================================================================================================
// DTB construction from compiling DTS
// ================================================================================================

pub fn dts_to_dtb(
    dts_path: &std::path::Path,
    include_dirs: &[&std::path::Path],
) -> Result<Vec<u8>, String> {
    let preprocessed_path = dts_path.with_extension("preprocessed.dts");
    let dtb_path = dts_path.with_extension("dtb");

    // stage 1 - preprocessing
    // -E: preprocess only
    // -nostdinc: caller provides all needed headers
    // -undef: don't predefine macros
    // -x assembler-with-cpp: preprocessor interpreter
    let mut cpp_cmd = std::process::Command::new("cpp");
    cpp_cmd.args(["-E", "-nostdinc", "-undef", "-x", "assembler-with-cpp"]);

    for dir in include_dirs {
        cpp_cmd.arg("-I").arg(dir);
    }

    cpp_cmd.arg(dts_path).arg("-o").arg(&preprocessed_path);
    let cpp_status = cpp_cmd
        .status()
        .map_err(|e| format!("cpp not found: {e}. Install with: apt install gcc"))?;

    if !cpp_status.success() {
        return Err("cpp preprocessing failed".to_string());
    }

    // stage 2 - dts compilation
    let mut dtc_cmd = std::process::Command::new("dtc");
    dtc_cmd
        .arg("-I")
        .arg("dts")
        .arg("-O")
        .arg("dtb")
        .arg("-o")
        .arg(&dtb_path)
        .arg(&preprocessed_path);
    let dtc_status = dtc_cmd
        .status()
        .map_err(|e| {
            format!("dtc not found: {e}. Install with: apt install device-tree-compiler")
        })?;

    if !dtc_status.success() {
        return Err("dtc failed".to_string());
    }

    std::fs::read(&dtb_path).map_err(|e| format!("cannot read DTB: {e}"))
}

// ================================================================================================
// DeviceTree construction from walk through DTB in-memory blob via FDT crate
// ================================================================================================

pub fn dtb_to_devicetree(dtb: &[u8]) -> Result<DeviceTree, String> {
    let fdt = fdt::Fdt::new(dtb).map_err(|e| format!("fdt parse error: {e}"))?;
    let mut tree = DeviceTree {
        nodes: Vec::new(),
        by_phandle: HashMap::new(),
        by_name: HashMap::new(),
        root: 0,
    };

    let root = fdt.find_node("/").ok_or("cannot find root node")?;
    let addr_cells = read_cell_count(&root, "#address-cells").unwrap_or(1);
    let size_cells = read_cell_count(&root, "#size-cells").unwrap_or(1);

    tree.root = walk(root, None, &mut tree, addr_cells, size_cells);
    Ok(tree)
}

fn walk<'a>(
    node: fdt::node::FdtNode<'a, '_>,
    parent: Option<usize>,
    tree: &mut DeviceTree,
    addr_cells: u32,
    size_cells: u32,
) -> usize {
    let name = node.name.to_string();

    let compatible: Vec<String> = node
        .compatible()
        .map(|c| c.all().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let phandle = node
        .property("phandle")
        .filter(|p| p.value.len() == 4)
        .map(|p| u32::from_be_bytes(p.value.try_into().unwrap()));

    let child_addr_cells = read_cell_count(&node, "#address-cells").unwrap_or(addr_cells);
    let child_size_cells = read_cell_count(&node, "#size-cells").unwrap_or(size_cells);

    let reg = parse_reg(&node, addr_cells, size_cells);
    let interrupts: Vec<u32> = node
        .property("interrupts")
        .map(|p| {
            p.value
                .chunks(4)
                .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
                .collect()
        })
        .unwrap_or_default();

    const SKIP: &[&str] = &[
        "compatible",
        "reg",
        "phandle",
        "linux,phandle",
        "interrupts",
        "#address-cells",
        "#size-cells",
        "name",
    ];

    let mut extra = HashMap::new();
    for prop in node.properties() {
        if SKIP.contains(&prop.name) {
            continue;
        }
        extra.insert(prop.name.to_string(), parse_prop_value(prop.value));
    }

    let idx = tree.nodes.len();
    tree.nodes.push(Node {
        name: name.clone(),
        compatible,
        reg,
        interrupts,
        phandle,
        extra,
        children: Vec::new(),
        parent,
    });

    if let Some(ph) = phandle {
        tree.by_phandle.insert(ph, idx);
    }
    tree.by_name.insert(name, idx);

    for child in node.children() {
        let child_idx = walk(child, Some(idx), tree, child_addr_cells, child_size_cells);
        tree.nodes[idx].children.push(child_idx);
    }

    idx
}

// ================================================================================================
// Helpers
// ================================================================================================

fn read_cell_count<'a>(node: &fdt::node::FdtNode<'a, '_>, prop: &str) -> Option<u32> {
    node.property(prop)
        .filter(|p| p.value.len() == 4)
        .map(|p| u32::from_be_bytes(p.value.try_into().unwrap()))
}

fn parse_reg<'a>(
    node: &fdt::node::FdtNode<'a, '_>,
    addr_cells: u32,
    size_cells: u32,
) -> Option<(u64, u64)> {
    let prop = node.property("reg")?;
    let words: Vec<u32> = prop
        .value
        .chunks(4)
        .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
        .collect();

    let addr = match addr_cells {
        1 => *words.first()? as u64,
        2 => ((*words.first()? as u64) << 32) | *words.get(1)? as u64,
        _ => return None,
    };

    let size = match size_cells {
        0 => 0u64,
        1 => *words.get(addr_cells as usize)? as u64,
        2 => {
            let i = addr_cells as usize;
            ((*words.get(i)? as u64) << 32) | *words.get(i + 1)? as u64
        }
        _ => return None,
    };

    Some((addr, size))
}

fn parse_prop_value(bytes: &[u8]) -> PropValue {
    if bytes.is_empty() {
        return PropValue::Empty;
    }

    if bytes.last() == Some(&0) {
        let is_printable_ascii = bytes[..bytes.len() - 1]
            .iter()
            .all(|&b| b == 0 || (b >= 0x20 && b <= 0x7e));

        if is_printable_ascii {
            let s = std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap();
            let parts: Vec<&str> = s.split('\0').collect();
            return if parts.len() == 1 {
                PropValue::Str(parts[0].to_string())
            } else {
                PropValue::StringList(parts.iter().map(|s| s.to_string()).collect())
            };
        }
    }

    if bytes.len().is_multiple_of(4) {
        let words: Vec<u32> = bytes
            .chunks(4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
            .collect();
        return if words.len() == 1 {
            PropValue::U32(words[0])
        } else {
            PropValue::U32Array(words)
        };
    }

    PropValue::Bytes(bytes.to_vec())
}
