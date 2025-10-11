
#include <stdint.h>
#include "mem.h"

#include <kernel/lib.h>

extern uintptr_t __bss_start;
extern uintptr_t __bss_end;

extern uintptr_t __data_start;
extern uintptr_t __data;
extern uintptr_t __data_end;

extern uintptr_t __got_start;
extern uintptr_t __got_end;
extern uintptr_t __got_load;

extern uintptr_t __rel_dyn_start;
extern uintptr_t __rel_dyn_end;
extern uintptr_t __rel_dyn_load;

typedef void (*func_t)(void);

extern func_t __init_array_start;
extern func_t __init_array_end;
extern func_t __fini_array_start;
extern func_t __fini_array_end;

extern void _main(void) __attribute__((noreturn));
extern void init_boot_info(BootInfo *boot_info);

extern int main(void);

void call_constructors(void)
{
    for (func_t *func = &__init_array_start; func < &__init_array_end; func++)
    {
        (*func)();
    }
}

void call_destructors(void)
{
    for (func_t *func = &__fini_array_start; func < &__fini_array_end; func++)
    {
        (*func)();
    }
}

// Structure for ELF32 relocation entries
typedef struct {
    uint32_t r_offset;
    uint32_t r_info;
} Elf32_Rel;

#define ELF32_R_TYPE(info) ((info) & 0xff)
#define R_ARM_RELATIVE 23

// Relocate GOT entries for position-independent code
void relocate_got(void)
{
    // Copy GOT from FLASH to RAM
    size_t got_len = (uintptr_t)&__got_end - (uintptr_t)&__got_start;
    if (got_len > 0)
    {
        memcpy(&__got_start, &__got_load, got_len);
    }

    // Copy relocation table from FLASH to RAM
    size_t rel_dyn_len = (uintptr_t)&__rel_dyn_end - (uintptr_t)&__rel_dyn_start;
    if (rel_dyn_len > 0)
    {
        memcpy(&__rel_dyn_start, &__rel_dyn_load, rel_dyn_len);
    }

    // Calculate the actual load address offset
    // For ARM Cortex-M, we can read the vector table location from VTOR (if available)
    // or from SCB->VTOR. For now, we use the fact that vector table is at the start.
    // In a fully relocatable system, the offset would be determined at load time.
    extern uintptr_t __vector_table;
    
    // For initial implementation, assume no offset (binary loaded at expected address)
    // In a real relocatable system, this would be passed by the bootloader
    intptr_t offset = 0;
    
    // If we ever need to support relocation at runtime, the offset calculation would be:
    // offset = (actual_load_address) - (uintptr_t)&__vector_table;

    // Process R_ARM_RELATIVE relocations
    Elf32_Rel *rel = (Elf32_Rel *)&__rel_dyn_start;
    Elf32_Rel *rel_end = (Elf32_Rel *)&__rel_dyn_end;
    
    for (; rel < rel_end; rel++)
    {
        uint32_t type = ELF32_R_TYPE(rel->r_info);
        if (type == R_ARM_RELATIVE)
        {
            uint32_t *ptr = (uint32_t *)rel->r_offset;
            *ptr += offset;
        }
    }
}

void _main(void)
{
    // zero bss section
    size_t bss_len = (uintptr_t)&__bss_end - (uintptr_t)&__bss_start;

    if (bss_len > 0)
    {
        memset(&__bss_start, 0, bss_len);
    }

    // copy data section
    size_t data_len = (uintptr_t)&__data_end - (uintptr_t)&__data_start;

    if (data_len > 0)
    {
        memcpy(&__data_start, &__data, data_len);
    }

    // Relocate GOT for position-independent code
    relocate_got();

    call_constructors();

    // Init boot info
    BootInfo boot_info;
    memset(&boot_info, 0, sizeof(BootInfo));
    init_boot_info(&boot_info);

    // Boot!
    kernel_init(&boot_info);
    unreachable();
}
