
#include <stdint.h>
#include <nlib/core.h>

extern uintptr_t __ram_start;

extern uintptr_t __bss_start;
extern uintptr_t __bss_end;

extern uintptr_t __data_start;
extern uintptr_t __data;
extern uintptr_t __data_end;

extern uintptr_t __ivt_start;
extern uintptr_t __ivt;
extern uintptr_t __ivt_end;

typedef void (*func_t)(void);

extern func_t __init_array_start;
extern func_t __init_array_end;
extern func_t __fini_array_start;
extern func_t __fini_array_end;

extern void _main(uint32_t) __attribute__((noreturn));

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

static inline void __DSB(void)
{
    __asm volatile ("dsb" ::: "memory");
}

void _main(uint32_t offset)
{
    // zero bss section
    size_t bss_len = (char*)&__bss_end - (char*)&__bss_start;

    if (bss_len > 0)
    {
        memset(&__bss_start, 0, bss_len);
    }

    // copy data section
    size_t data_len = (char*)&__data_end - (char*)&__data_start;

    if (data_len > 0)
    {
        memcpy(&__data_start, ((char*)(&__data)) + offset, data_len);
    }

    size_t ivt_len = (char*)&__ivt_end - (char*)&__ivt_start;

    if (ivt_len > 0 && offset != 0)
    {
        memcpy(&__ivt_start, ((char*)(&__ivt)) + offset, ivt_len);
        
        // relocate all function pointers in the IVT
        for (size_t i = 1; i < ivt_len / sizeof(uint32_t); i++)
        {
            ((uint32_t*)&__ivt_start)[i] += offset;
        }

        // set the vtor offset register
        *((volatile uint32_t*)0xE000ED08) = &__ivt_start;

        __DSB();
    }

    call_constructors();

    // call main
    main();
    unreachable();
}
