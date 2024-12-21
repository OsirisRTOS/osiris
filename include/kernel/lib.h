#ifndef KERNEL_H
#define KERNEL_H

#include "stdint.h"
#include "stdbool.h"
#include "stdarg.h"

void kernel_init(void);

void among(int32_t argc, const void *svc_args);

#endif /* KERNEL_H */
