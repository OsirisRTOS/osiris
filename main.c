#include <stdint.h>
#include <nlib/core.h>

int main(void)
{
    const char message[] = "Hello World!\n";

    semihosting_call(SYS_WRITE0, message);

    while (1)
    {
        __asm__("WFI");
    }

    return 0;
}