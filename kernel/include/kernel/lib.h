#ifndef KERNEL_H
#define KERNEL_H

#include "stdint.h"
#include "stdbool.h"
#include "stdarg.h"

/**
 * The memory map entry type.
 *
 * This structure shall be compatible with the multiboot_memory_map_t struct at
 * Link: [https://www.gnu.org/software/grub/manual/multiboot/multiboot.html]()
 */
typedef struct __attribute__((packed)) MemMapEntry {
  /**
   * The size of the entry.
   */
  uint32_t size;
  /**
   * The base address of the memory region.
   */
  uint64_t addr;
  /**
   * The length of the memory region.
   */
  uint64_t length;
  /**
   * The type of the memory region.
   */
  uint32_t ty;
} MemMapEntry;

/**
 * The boot information structure.
 */
typedef struct BootInfo {
  /**
   * The implementer of the processor.
   */
  const char *implementer;
  /**
   * The variant of the processor.
   */
  const char *variant;
  /**
   * The memory map.
   */
  struct MemMapEntry mmap[8];
  /**
   * The length of the memory map.
   */
  uintptr_t mmap_len;
} BootInfo;

/**
 * The kernel initialization function.
 *
 * `boot_info` - The boot information.
 */
void kernel_init(const struct BootInfo *boot_info);

#endif  /* KERNEL_H */
