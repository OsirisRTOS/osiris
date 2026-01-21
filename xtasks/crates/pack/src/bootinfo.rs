use crate::image;

pub struct BootInfo {
    inner: Vec<u8>,
}

impl BootInfo {
    pub fn new(img_paddr: usize, section: &image::Section) -> Self {
        let boot_info = interface::BootInfo {
            magic: interface::BOOT_INFO_MAGIC,
            version: 1,
            mmap: [interface::MemMapEntry {
                size: 0,
                addr: 0,
                length: 0,
                ty: 0,
            }; 8],
            mmap_len: 0,
            args: interface::Args {
                init: interface::InitDescriptor {
                    begin: (img_paddr + section.offset()) as u64,
                    len: section.size() as u64,
                    entry_offset: section.entry_offset() as u64,
                },
            },
        };

        let boot_info_bytes = bytemuck::bytes_of(&boot_info);

        Self {
            inner: boot_info_bytes.to_vec(),
        }
    }

    pub fn inner(&self) -> &Vec<u8> {
        &self.inner
    }
}

// Tests for bootinfo
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootinfo_fields() {
        let boot_info = BootInfo::new(
            0x4000,
            &image::Section::from_parts(0x4000, 0x2000, 0x100, 0x1000, 0, false),
        );

        // Deserialize back to struct for comparison
        assert_eq!(
            boot_info.inner().len(),
            std::mem::size_of::<interface::BootInfo>()
        );

        let reconstructed: interface::BootInfo =
            unsafe { std::ptr::read(boot_info.inner().as_ptr() as *const interface::BootInfo) };

        assert_eq!(reconstructed.magic, interface::BOOT_INFO_MAGIC);
        assert_eq!(reconstructed.version, 1);
        assert_eq!(reconstructed.args.init.begin, 0x4000 + 0x4000);
        assert_eq!(reconstructed.args.init.len, 0x2000);
        assert_eq!(reconstructed.args.init.entry_offset, 0x100);
    }
}
