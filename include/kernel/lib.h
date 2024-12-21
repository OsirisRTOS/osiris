#ifndef KERNEL_H
#define KERNEL_H

#include "stdint.h"
#include "stdbool.h"
#include "stdarg.h"

void kernel_init(void);

void syscall_dummy(const void *svc_args);

#endif /* KERNEL_H */
