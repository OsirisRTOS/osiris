
#include <stdint.h>
#include "mem.h"

#include <kernel/lib.h>

extern uintptr_t __bss_start;
extern uintptr_t __bss_end;

extern uintptr_t __data_start;
extern uintptr_t __data;
extern uintptr_t __data_end;

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

    call_constructors();

    // Init boot info
    BootInfo boot_info;
    memset(&boot_info, 0, sizeof(BootInfo));
    init_boot_info(&boot_info);

    // Boot!
    kernel_init(&boot_info);
    unreachable();
}
