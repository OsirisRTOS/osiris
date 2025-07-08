#include <nlib/core.h>

#if defined(__GNUC__) || defined(__GNUG__)

void *__inhibit_loop_to_libcall memcpy(void *__restrict dst0, const void *__restrict src0, size_t len)
{
    char *dst = (char *)dst0;
    char *src = (char *)src0;

    void *save = dst0;

    while (len--)
    {
        *dst++ = *src++;
    }

    return save;
}

void *__inhibit_loop_to_libcall memset(void *dst0, int c, size_t len)
{
    char *dst = (char *)dst0;

    while (len--)
        *dst++ = (char)c;

    return dst;
}

void *__inhibit_loop_to_libcall memmove(void *dst0, const void *src0, size_t len)
{
    char *dst = dst0;
    const char *src = src0;

    if (src < dst && dst < src + len)
    {
        src += len;
        dst += len;
        while (len--)
        {
            *--dst = *--src;
        }
    }
    else
    {
        while (len--)
        {
            *dst++ = *src++;
        }
    }

    return dst0;
}
#endif // defined(__GNUC__) || defined(__GNUG__)