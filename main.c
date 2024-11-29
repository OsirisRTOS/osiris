#include <stdint.h>

int semihosting_call(int reason, const void *arg);

#define SYS_WRITE0 0x04

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

int semihosting_call(int reason, const void *arg)
{
    int result;

    __asm__ volatile(
        "mov r0, %[rsn]\n"
        "mov r1, %[arg]\n"
        "bkpt 0xAB\n"
        "mov %[res], r0\n"
        : [res] "=r"(result)
        : [rsn] "r"(reason), [arg] "r"(arg)
        : "r0", "r1", "memory");

    return result;
}