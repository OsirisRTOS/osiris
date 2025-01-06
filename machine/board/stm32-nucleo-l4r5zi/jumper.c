#include <stdint.h>

#include "../../../kernel/include/kernel/lib.h"

extern uint32_t get_mem_map(MemMapEntry *mem_map, uint32_t max_size);

extern uint32_t get_mem_map(MemMapEntry *mem_map, uint32_t max_size)
{
    BootInfo boot_info;

    boot_info.implementer = "ARM";
    boot_info.variant = "Cortex-M4";

    // SRAM1
    mem_map[0] = (MemMapEntry){
        .size = sizeof(MemMapEntry),
        .addr = 0x20000000,
        .length = 0x30000,
        .ty = 1};

    // SRAM2
    mem_map[1] = (MemMapEntry){
        .size = sizeof(MemMapEntry),
        .addr = 0x20030000,
        .length = 0x100000,
        .ty = 1};

    // SRAM3
    mem_map[2] = (MemMapEntry){
        .size = sizeof(MemMapEntry),
        .addr = 0x20040000,
        .length = 0x60000,
        .ty = 1};

    return 3;
}