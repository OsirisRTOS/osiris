#ifndef KERNEL_H
#define KERNEL_H

#include "stdint.h"
#include "stdbool.h"
#include "stdarg.h"

/**
 * The memory map entry type.
 *
 * This structure shall be compatible with the multiboot_memory_map_t struct at
 * Link: https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
 */
typedef struct __attribute__((packed)) MemMapEntry {
  uint32_t size;
  uint64_t addr;
  uint64_t length;
  uint32_t ty;
} MemMapEntry;

typedef struct BootInfo {
  const char *implementer;
  const char *variant;
  const struct MemMapEntry *mem_map;
  uintptr_t mem_map_len;
} BootInfo;

void kernel_init(struct BootInfo boot_info);

void syscall_dummy(const void *svc_args);

#endif /* KERNEL_H */
