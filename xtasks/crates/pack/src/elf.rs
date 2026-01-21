use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use elf::endian::AnyEndian;

#[derive(Debug, Clone)]
pub struct ElfInfo {
    inner: Vec<u8>,
    path: PathBuf,
    base_paddr: usize,
    entry_offset: usize,
    align: usize,
    size: usize,
}

#[allow(dead_code)]
impl ElfInfo {
    pub fn from_path(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read ELF file at {}", path.display()))?;

        let file = elf::ElfBytes::<AnyEndian>::minimal_parse(&data)
            .with_context(|| format!("Failed to parse ELF file at {}", path.display()))?;

        let entry_point = file.ehdr.e_entry as usize;

        let entry_point = Self::vaddr_to_paddr(&file, entry_point).with_context(|| {
            format!(
                "Failed to convert entry point vaddr to paddr for ELF at {}",
                path.display()
            )
        })?;

        let (pstart, size) = Self::calc_load_size(&file);
        let align = Self::calc_align(&file);

        let base_paddr = Self::calc_base_paddr(&file).with_context(|| {
            format!(
                "Failed to calculate base physical address for ELF at {}",
                path.display()
            )
        })?;

        Ok(Self {
            inner: data,
            path: path.to_path_buf(),
            base_paddr,
            entry_offset: entry_point - pstart,
            align,
            size,
        })
    }

    fn calc_load_size(bytes: &elf::ElfBytes<AnyEndian>) -> (usize, usize) {
        let mut min: Option<usize> = None;
        let mut max: Option<usize> = None;

        bytes.segments().inspect(|pt| {
            for ph in pt.iter() {
                if ph.p_type != elf::abi::PT_LOAD {
                    continue;
                }

                let start = ph.p_paddr as usize;
                let end = ph.p_paddr.saturating_add(ph.p_memsz) as usize;

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
            (Some(min), Some(max)) => (min, max - min),
            _ => (0, 0),
        }
    }

    fn vaddr_to_paddr(bytes: &elf::ElfBytes<AnyEndian>, vaddr: usize) -> Option<usize> {
        for seg in bytes.segments()? {
            if seg.p_type != elf::abi::PT_LOAD {
                continue;
            }

            let seg_start = seg.p_vaddr as usize;
            let seg_end = seg.p_vaddr.saturating_add(seg.p_memsz) as usize;

            if vaddr >= seg_start && vaddr < seg_end {
                let offset = vaddr - seg_start;
                return Some(seg.p_paddr as usize + offset);
            }
        }

        None
    }

    fn calc_align(bytes: &elf::ElfBytes<AnyEndian>) -> usize {
        let mut max_align = 0x10;

        for seg in bytes.segments().into_iter().flatten() {
            if seg.p_type != elf::abi::PT_LOAD {
                continue;
            }

            if seg.p_align as usize > max_align {
                max_align = seg.p_align as usize;
            }
        }

        max_align
    }

    fn calc_base_paddr(bytes: &elf::ElfBytes<AnyEndian>) -> Option<usize> {
        let mut base: Option<usize> = None;

        for seg in bytes.segments().into_iter().flatten() {
            if seg.p_type != elf::abi::PT_LOAD {
                continue;
            }

            let paddr = seg.p_paddr as usize;

            base = Some(match base {
                Some(b) => b.min(paddr),
                None => paddr,
            });
        }

        base
    }

    pub fn add_to_image(&self, img: &mut Vec<u8>, base: usize) -> Result<()> {
        assert!(base.is_multiple_of(self.align()));

        img.extend(vec![0; (base + self.size()).saturating_sub(img.len())]);

        log::info!(
            "Placing ELF at {} into image at offset {:#x}, size {:#x}, total image size {:#x}",
            self.path.display(),
            base,
            self.size(),
            img.len()
        );

        self.place_into(&mut img[base..(base + self.size())])
    }

    pub fn place_into(&self, img: &mut [u8]) -> Result<()> {
        if img.len() != self.size() {
            bail!(
                "Image size {} does not match ELF size {} for ELF at {}",
                img.len(),
                self.size(),
                self.path.display()
            );
        }

        let parser = self.parser()?;

        let segs = parser.segments().with_context(|| {
            format!("Failed to get program headers for {}", self.path.display())
        })?;

        let mut segs: Vec<_> = segs
            .iter()
            .filter(|seg| seg.p_type == elf::abi::PT_LOAD)
            .collect();

        segs.sort_by_key(|seg| seg.p_paddr);
        let mut write_idx = 0;
        let mut current_paddr = self.base_paddr();

        for seg in segs {
            assert!(seg.p_paddr as usize >= current_paddr);

            let padding = seg.p_paddr as usize - current_paddr;

            // Pad to segment paddr
            img[write_idx..write_idx + padding].fill(0);
            write_idx += padding;

            // Copy segment data
            let file_offset = seg.p_offset as usize;
            log::info!(
                "  Placing segment at paddr {:#x}, offset {:#x}, filesz {:#x}, memsz {:#x}",
                seg.p_paddr,
                seg.p_offset,
                seg.p_filesz,
                seg.p_memsz
            );
            let data_size = seg.p_filesz as usize;

            assert!(write_idx + self.base_paddr() == seg.p_paddr as usize);

            img[write_idx..write_idx + data_size]
                .copy_from_slice(&self.inner()[file_offset..file_offset + data_size]);
            write_idx += data_size;

            // Pad to memsz
            let mem_size = seg.p_memsz as usize;

            img[write_idx..write_idx + (mem_size - data_size)].fill(0);

            current_paddr = seg.p_vaddr as usize + seg.p_memsz as usize;
        }

        Ok(())
    }

    pub fn patch_section(&mut self, section_name: &str, offset: usize, data: &[u8]) -> Result<()> {
        let elf = self.parser()?;

        let mut target = None;

        let sections = elf.section_headers_with_strtab().with_context(|| {
            format!(
                "Failed to get section headers with strtab for ELF at {}",
                self.path.display()
            )
        })?;

        if let (Some(shdrs), Some(strtab)) = sections {
            for sh in shdrs.iter() {
                if let Ok(name) = strtab.get(sh.sh_name as usize)
                    && name == section_name
                {
                    target = Some(sh);
                    break;
                }
            }
        }

        let section = match target {
            Some(s) => s,
            None => {
                bail!(
                    "Failed to find section {} in ELF at {}",
                    section_name,
                    self.path.display()
                );
            }
        };

        if section.sh_type == elf::abi::SHT_NOBITS {
            bail!(
                "Cannot patch data into SHT_NOBITS section {} in ELF at {}",
                section_name,
                self.path.display()
            );
        }

        let section_offset = section.sh_offset as usize;
        let section_size = section.sh_size as usize;

        if offset.saturating_add(data.len()) > section_size {
            bail!(
                "Data to patch exceeds section {} size in ELF at {}",
                section_name,
                self.path.display()
            );
        }

        let file_start = section_offset.checked_add(offset).with_context(|| {
            format!(
                "Overflow calculating patch offset in section {} in ELF at {}",
                section_name,
                self.path.display()
            )
        })?;

        let file_end = file_start.checked_add(data.len()).with_context(|| {
            format!(
                "Overflow calculating patch end in section {} in ELF at {}",
                section_name,
                self.path.display()
            )
        })?;

        if file_end > self.inner.len() {
            bail!(
                "Patch end exceeds file size in ELF at {}",
                self.path.display()
            );
        }

        log::info!(
            "Patching {} bytes into section {} at offset {:#x} in ELF at {}",
            data.len(),
            section_name,
            file_end,
            self.path.display()
        );

        self.inner[file_start..file_end].copy_from_slice(data);
        Ok(())
    }

    pub fn parser(&self) -> Result<elf::ElfBytes<'_, AnyEndian>> {
        elf::ElfBytes::<AnyEndian>::minimal_parse(&self.inner)
            .with_context(|| format!("Failed to parse ELF file at {}", self.path.display()))
    }

    pub fn inner(&self) -> &Vec<u8> {
        &self.inner
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn base_paddr(&self) -> usize {
        self.base_paddr
    }

    pub fn align(&self) -> usize {
        self.align
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn entry_offset(&self) -> usize {
        self.entry_offset
    }
}

impl Display for ElfInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ELF Info:")?;
        writeln!(f, "  Path: {}", self.path.display())?;
        writeln!(f, "  Base PAddr: {:#x}", self.base_paddr)?;
        writeln!(f, "  Entry Offset: {:#x}", self.entry_offset)?;
        writeln!(f, "  Align: {:#x}", self.align)?;
        writeln!(f, "  Size: {:#x}", self.size)?;
        Ok(())
    }
}
