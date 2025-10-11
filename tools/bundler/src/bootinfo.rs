use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use elf::endian::AnyEndian;

pub struct ElfInfo {
    inner: Vec<u8>,
    path: PathBuf,
    entry_offset: usize,
    size: usize,
}

impl ElfInfo {
    fn calc_load_size(bytes: &elf::ElfBytes<AnyEndian>) -> usize {
        let mut min: Option<usize> = None;
        let mut max: Option<usize> = None;

        bytes.segments().inspect(|pt| {
            for ph in pt.iter() {
                if ph.p_type != elf::abi::PT_LOAD {
                    continue;
                }

                let start = ph.p_vaddr as usize;
                let end = ph.p_vaddr.saturating_add(ph.p_memsz) as usize;

                if end <= start {
                    continue;
                }

                min = Some(match min {
                    Some(m) => m.min(start),
                    None => start,
                });

                max = Some(match max {
                    Some(m) => m.max(end),
                    None => end,
                });
            }
        });

        match (min, max) {
            (Some(min), Some(max)) => max - min,
            _ => 0,
        }
    }

    fn parser(&self) -> Result<elf::ElfBytes<AnyEndian>> {
        elf::ElfBytes::<AnyEndian>::minimal_parse(&self.inner).with_context(|| {
            format!(
                "Failed to parse ELF file at {}",
                self.path.to_string_lossy()
            )
        })
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read ELF file at {}", path.to_string_lossy()))?;

        let file = elf::ElfBytes::<AnyEndian>::minimal_parse(&data)
            .with_context(|| format!("Failed to parse ELF file at {}", path.to_string_lossy()))?;

        let entry_offset = file.ehdr.e_entry as usize;
        // Size is the size of the resulting binary
        let size = Self::calc_load_size(&file);

        Ok(Self {
            inner: data,
            path: path.to_path_buf(),
            entry_offset,
            size,
        })
    }

    fn patch_bytes(
        &self,
        section_name: &str,
        offset: usize,
        data: &[u8],
        out: &Path,
    ) -> Result<()> {
        let elf = self.parser()?;

        let mut target = None;

        let sections = elf.section_headers_with_strtab().with_context(|| {
            format!(
                "Failed to get section headers with strtab for ELF at {}",
                self.path.to_string_lossy()
            )
        })?;

        if let (Some(shdrs), Some(strtab)) = sections {
            for sh in shdrs.iter() {
                if let Ok(name) = strtab.get(sh.sh_name as usize) {
                    if name == section_name {
                        target = Some(sh);
                        break;
                    }
                }
            }
        }

        let section = match target {
            Some(s) => s,
            None => {
                bail!(
                    "Failed to find section {} in ELF at {}",
                    section_name,
                    self.path.to_string_lossy()
                );
            }
        };

        if section.sh_type == elf::abi::SHT_NOBITS {
            bail!(
                "Cannot patch data into SHT_NOBITS section {} in ELF at {}",
                section_name,
                self.path.to_string_lossy()
            );
        }

        let section_offset = section.sh_offset as usize;
        let section_size = section.sh_size as usize;

        if offset.saturating_add(data.len()) > section_size {
            bail!(
                "Data to patch exceeds section {} size in ELF at {}",
                section_name,
                self.path.to_string_lossy()
            );
        }

        let file_start = section_offset.checked_add(offset).with_context(|| {
            format!(
                "Overflow calculating patch offset in section {} in ELF at {}",
                section_name,
                self.path.to_string_lossy()
            )
        })?;

        let file_end = file_start.checked_add(data.len()).with_context(|| {
            format!(
                "Overflow calculating patch end in section {} in ELF at {}",
                section_name,
                self.path.to_string_lossy()
            )
        })?;

        if file_end > self.inner.len() {
            bail!(
                "Patch end exceeds file size in ELF at {}",
                self.path.to_string_lossy()
            );
        }

        let mut inner = self.inner.clone();
        inner[file_start..file_end].copy_from_slice(data);

        fs::write(out, &inner).with_context(|| {
            format!(
                "Failed to write patched ELF file at {}",
                self.path.to_string_lossy()
            )
        })?;

        Ok(())
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

pub fn inject_bootinfo(kernel_info: &ElfInfo, app_base: usize, app_info: &ElfInfo, out: &Path) -> Result<()> {
    // Prepare the BootInfo structure
    let boot_info = interface::BootInfo {
        magic: interface::BOOT_INFO_MAGIC,
        version: 1,
        implementer: std::ptr::null(),
        variant: std::ptr::null(),
        mmap: [interface::MemMapEntry {
            size: 0,
            addr: 0,
            length: 0,
            ty: 0,
        }; 8],
        mmap_len: 0,
        args: interface::Args {
            init: interface::InitDescriptor {
                begin: (app_base) as *const usize,
                len: app_info.size,
                entry_offset: app_info.entry_offset,
            },
        },
    };

    let boot_info_bytes = unsafe {
        std::slice::from_raw_parts(
            &boot_info as *const interface::BootInfo as *const u8,
            std::mem::size_of::<interface::BootInfo>(),
        )
    };

    kernel_info.patch_bytes(".bootinfo", 0, boot_info_bytes, out)?;
    Ok(())
}
