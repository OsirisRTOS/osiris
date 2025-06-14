#include <stdint.h>

#include "../../../../kernel/include/kernel/lib.h"

extern void init_boot_info(BootInfo *boot_info);

extern void init_boot_info(BootInfo *boot_info)
{
    boot_info->implementer = "ARM";
    boot_info->variant = "Cortex-M4";

    boot_info->mmap;

    boot_info->mmap_len = 3;

    // SRAM1
    boot_info->mmap[0] = (MemMapEntry){
        .size = sizeof(MemMapEntry),
        .addr = 0x20000000,
        .length = 0x30000,
        .ty = 1};

    // SRAM2
    boot_info->mmap[1] = (MemMapEntry){
        .size = sizeof(MemMapEntry),
        .addr = 0x20030000,
        .length = 0x10000,
        .ty = 1};

    // SRAM3
    boot_info->mmap[2] = (MemMapEntry){
        .size = sizeof(MemMapEntry),
        .addr = 0x20040000,
        .length = 0x60000,
        .ty = 1};
}