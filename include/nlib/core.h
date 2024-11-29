#ifndef CORE_H
#define CORE_H

#include <stddef.h>

/* This is a minimal implementation of functions needed to run a C program. */

#if defined(__GNUC__) || defined(__GNUG__)

#define __inhibit_loop_to_libcall __attribute__((optimize("no-tree-loop-distribute-patterns")))
void *__inhibit_loop_to_libcall memcpy(void *__restrict dst0, const void *__restrict src0, size_t len);
void *__inhibit_loop_to_libcall memset(void *dst0, int c, size_t len);
void *__inhibit_loop_to_libcall memmove(void *dst0, const void *src0, size_t len);

#elif defined(__clang__)

#define memcpy(dst, src, len) __builtin_memcpy_inline(dst, src, len)
#define memset(dst, c, len) __builtin_memset_inline(dst, c, len)
#define memmove(dst, src, len) __builtin_memmove_inline(dst, src, len)

#endif // defined(__GNUC__) || defined(__GNUG__)

#define SYS_WRITE0 0x04
int semihosting_call(int reason, const void *arg);

#endif // CORE_H
