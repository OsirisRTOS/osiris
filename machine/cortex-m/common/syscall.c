
/*
 * This code has been taken from the following example:
 * https://developer.arm.com/documentation/ka004005/latest/
 *
 */

extern void _syscall_hndlr(unsigned int *svc_args);

extern int handle_syscall(unsigned int svc_number, unsigned int *svc_args);

void _syscall_hndlr(unsigned int *svc_args)
{
    unsigned int svc_number;
    svc_number = ((char *)svc_args[6])[-2];
    
    int ret = handle_syscall(svc_number, svc_args);
    svc_args[0] = ret;
}