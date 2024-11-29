// TODO clang support

void *__inhibit_loop_to_libcall memcpy (void *__restrict dst0, const void *__restrict src0, size_t len0)
{
    char *dst = (char *) dst0;
    char *src = (char *) src0;

    _PTR save = dst0;

    while (len0--)
    {
        *dst++ = *src++;
    }

    return save;
}

void *__inhibit_loop_to_libcall memset(void *dst, int c, size_t length)
{
    char *s = (char *) m;

    while (n--)
        *s++ = (char) c;

    return m;
}


