use std::path::Path;

use anyhow::Result;

use crate::bootinfo::{self, ElfInfo};

fn find_elf(name: &str, path: &Path, release: bool, target: &Option<String>) -> Result<ElfInfo> {
    let mut search_paths = vec![path.to_path_buf()];

    if let Some(t) = target {
        if release {
            search_paths.push(path.join("target").join(t).join("release"));
        } else {
            search_paths.push(path.join("target").join(t).join("debug"));
        }
    }

    for p in search_paths {
        let candidate = p.join(name);
        if candidate.exists() {
            return ElfInfo::from_path(&candidate);
        }
    }

    anyhow::bail!("Failed to find ELF file named {}.", name);
}

fn align_up(addr: usize, align: usize) -> usize {
    if addr % align == 0 {
        return addr;
    }

    (addr + align - 1) & !(align - 1)
}

const MIN_ALIGN: usize = 0x10;

pub fn assemble(target: &Option<String>, kernel: &Path, app: &Path, out: &Path, release: bool) -> Result<()> {
    let kernel_info = find_elf("Kernel", kernel, release, target)?;
    let app_info = find_elf("App", app, release, target)?;

    let app_base = align_up(kernel_info.size(), MIN_ALIGN);

    bootinfo::inject_bootinfo(&kernel_info, app_base, &app_info, out)
}