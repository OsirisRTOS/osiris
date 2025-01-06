
#include <stdint.h>
#include <nlib/core.h>

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
    memset(&__bss_start, 0, (uintptr_t)&__bss_end - (uintptr_t)&__bss_start);

    // copy data section
    memcpy(&__data_start, &__data, (uintptr_t)&__data_end - (uintptr_t)&__data_start);

    call_constructors();

    // call main
    main();
    unreachable();
}
