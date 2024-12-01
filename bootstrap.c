#include <stdint.h>
#include <nlib/core.h>

extern void kernel_init(void);

int main(void)
{
    kernel_init();

    while (1)
    {
        __asm__("WFI");
    }
}
