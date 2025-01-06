#include <stdint.h>
#include <nlib/core.h>
#include <kernel/lib.h>

#define MEM_MAP_CAPACITY 32

extern uint32_t get_mem_map(MemMapEntry *mem_map, uint32_t max_size);

int main(void)
{
    BootInfo boot_info;
    boot_info.implementer = "ARM";
    boot_info.variant = "Cortex-M4";
    MemMapEntry mem_map[MEM_MAP_CAPACITY];

    uint32_t entries = get_mem_map(mem_map, MEM_MAP_CAPACITY);

    boot_info.mem_map = mem_map;
    boot_info.mem_map_len = entries;

    kernel_init(boot_info);
    unreachable();
}
