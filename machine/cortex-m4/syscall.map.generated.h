#include <stdint.h>

#define DECLARE_SYSCALL(name, number, argc) \
    case number:                            \
        name(argc, svc_args);               \
        break;

#define DECLARE_SYSCALLS()                                    \
    extern void reset(uint32_t argc, unsigned int *svc_args); \
    extern void among(uint32_t argc, unsigned int *svc_args);

#define IMPLEMENT_SYSCALLS()     \
    DECLARE_SYSCALL(reset, 0, 0) \
    DECLARE_SYSCALL(among, 1, 1)
