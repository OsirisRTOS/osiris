
/*
 * This code has been taken from the following example:
 * https://developer.arm.com/documentation/ka004005/latest/
 *
 */

#include <syscalls.map.gen.h>

extern void _syscall_hndlr(unsigned int *svc_args);

DECLARE_SYSCALLS()

// Handles all syscall requests.
void _syscall_hndlr(unsigned int *svc_args)
{
    unsigned int svc_number;
    svc_number = ((char *)svc_args[6])[-2];
    switch (svc_number)
    {
        IMPLEMENT_SYSCALLS()
    }
}