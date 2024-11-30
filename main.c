#include <stdint.h>
#include <nlib/core.h>
#include <hal/lib.h>

int main(void)
{
    hw_init();

    const char message[] = "Hello World!\n";

    semihosting_call(SYS_WRITE0, message);

    while (1)
    {
        __asm__("WFI");
    }

    return 0;
}
