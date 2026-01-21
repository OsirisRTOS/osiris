
#include <stdint.h>
#include "mem.h"

#include <bindings.h>

typedef void (*func_t)(void);

extern func_t __init_array_start;
extern func_t __init_array_end;
extern func_t __fini_array_start;
extern func_t __fini_array_end;

extern void pre_init(void) __attribute__((noreturn));
extern void init_mem_maps(BootInfo *boot_info);

__attribute__((section(".bootinfo"), used, aligned(4)))
static BootInfo _boot_info = {
    .magic = BOOT_INFO_MAGIC,
    .version = 1,
    .mmap = {0},
    .mmap_len = 0,
    .args = {.init = {0}},
};

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

void pre_init(void)
{
    // Init memory maps, etc.
    init_mem_maps(&_boot_info);

    // Boot!
    kernel_init(&_boot_info);
    unreachable();
}
