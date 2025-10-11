
#include <stdint.h>
#include "mem.h"

#include <bindings.h>

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

__attribute__((section(".bootinfo"), used, aligned(4)))
BootInfo _boot_info = {
    .magic = BOOT_INFO_MAGIC,
    .version = 1,
    .implementer = "Unknown",
    .variant = "Unknown",
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

void _main(void)
{
    // Inline asm that sets r9 to &__data
    __asm__ volatile ("mov r9, %0" :: "r"(&__data_start) : "r9");

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

    // We need a full barrier here to ensure that all operations are completed
    // and we can safely access global variables like _boot_info.
    __sync_synchronize();

    // Now we can actually access bootinfo.
    init_boot_info(&_boot_info);

    // Boot!
    kernel_init(&_boot_info);
    unreachable();
}
