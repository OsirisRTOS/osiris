#include "stdint.h"
#include "stdbool.h"
#include "stdarg.h"

#define BOOT_INFO_MAGIC 221566477

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

typedef struct InitDescriptor {
  /**
   * Pointer to the start of the binary of the init program.
   */
  const uintptr_t *begin;
  /**
   * Length of the binary of the init program.
   */
  uintptr_t len;
  uintptr_t entry_offset;
} InitDescriptor;

typedef struct Args {
  struct InitDescriptor init;
} Args;

/**
 * The boot information structure.
 */
typedef struct BootInfo {
  /**
   * The magic number that indicates valid boot information.
   */
  uint32_t magic;
  /**
   * The version of the boot information structure.
   */
  uint32_t version;
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
  /**
   * The command line arguments.
   */
  struct Args args;
} BootInfo;

extern void kernel_init(const struct BootInfo *boot_info);
