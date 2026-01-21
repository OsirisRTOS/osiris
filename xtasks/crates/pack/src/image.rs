use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::elf::ElfInfo;

/// Represents a section within a bundled image
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Section {
    /// Address of where the section is in the image
    offset: usize,
    /// Size of the section
    size: usize,
    /// Offset to the entry point within the section
    entry_offset: usize,
    /// When the section is relocatable, this is the alignment requirement that the new address must meet
    align: usize,
    /// CRC32 checksum of the section data
    crc: u32,
    /// Whether the section is relocatable
    relocatable: bool,
}

impl Section {
    pub fn new(elf: &ElfInfo, typ: SectionDescripter, base: usize, offset: usize) -> Result<Self> {
        match typ {
            SectionDescripter::Fixed => {
                let crc: u32 =
                    crc_fast::checksum(crc_fast::CrcAlgorithm::Crc32IsoHdlc, elf.inner()) as u32;

                let section = Self {
                    offset: elf.base_paddr() - base,
                    size: elf.size(),
                    entry_offset: elf.entry_offset(),
                    align: elf.align(),
                    crc,
                    relocatable: false,
                };

                Ok(section)
            }
            SectionDescripter::Loadable(load_offset) => {
                let crc: u32 =
                    crc_fast::checksum(crc_fast::CrcAlgorithm::Crc32IsoHdlc, elf.inner()) as u32;

                let offset = match load_offset {
                    Some(b) => b,
                    None => align_up(offset, elf.align()),
                };

                let section = Self {
                    offset,
                    size: elf.size(),
                    entry_offset: elf.entry_offset(),
                    align: elf.align(),
                    crc,
                    relocatable: true,
                };

                Ok(section)
            }
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn entry_offset(&self) -> usize {
        self.entry_offset
    }

    #[cfg(test)]
    pub fn from_parts(
        offset: usize,
        size: usize,
        entry_offset: usize,
        align: usize,
        crc: u32,
        relocatable: bool,
    ) -> Self {
        Self {
            offset,
            size,
            entry_offset,
            align,
            crc,
            relocatable,
        }
    }
}

/// Describes the type of section to add to an image
pub enum SectionDescripter {
    /// A section with a fixed physical address
    Fixed,
    /// A loadable section that may be relocated
    Loadable(Option<usize>), // optional base physical address
}

/// Represents a bundled image containing multiple ELF files
pub struct Image {
    /// Sections that describe the image contents
    sections: Vec<Section>,
    /// The physical address of the image start
    paddr: usize,
    /// The raw image data
    data: Vec<u8>,
}

#[allow(dead_code)]
impl Image {
    pub fn new(paddr: usize) -> Self {
        Self {
            sections: Vec::new(),
            paddr,
            data: Vec::new(),
        }
    }

    pub fn add_section(&mut self, elf: &ElfInfo, desc: SectionDescripter) -> Result<Section> {
        let offset = self.sections.last().map_or(0, |s| s.offset + s.size);
        log::info!(
            "Adding section: offset={:#x}, size={:#x}, align={}",
            offset,
            elf.size(),
            elf.align()
        );

        let section = Section::new(elf, desc, self.paddr, offset)?;
        self.sections.push(section.clone());
        Ok(section)
    }

    pub fn add_elf(&mut self, elf: &ElfInfo, typ: SectionDescripter) -> Result<Section> {
        let section = self.add_section(elf, typ)?;
        elf.add_to_image(&mut self.data, section.offset)?;
        Ok(section)
    }

    pub fn update(&mut self, elf: &ElfInfo, section_index: usize) -> Result<()> {
        let section = self.sections.get(section_index).with_context(|| {
            format!("Failed to get section at index {section_index} for update.")
        })?;

        if elf.align() > section.align {
            bail!(
                "New ELF alignment {} is greater than old alignment {}.",
                elf.align(),
                section.align
            );
        }

        if elf.size() > section.size {
            bail!(
                "New ELF size {} is greater than old size {}.",
                elf.size(),
                section.size
            );
        }

        elf.add_to_image(&mut self.data, section.offset)
    }

    pub fn write(&self, out: &Path) -> Result<()> {
        fs::write(out, &self.data)
            .with_context(|| format!("Failed to write image to {}", out.display()))
    }

    pub fn paddr(&self) -> usize {
        self.paddr
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    if addr.is_multiple_of(align) {
        return addr;
    }

    (addr + align - 1) & !(align - 1)
}
