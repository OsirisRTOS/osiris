#include <stdint.h>
#include <nlib/core.h>
#include <kernel/lib.h>

#define MEM_MAP_CAPACITY 32

extern void init_boot_info(BootInfo *boot_info);

int main(void)
{
    BootInfo boot_info;
    memset(&boot_info, 0, sizeof(BootInfo));
    init_boot_info(&boot_info);

    kernel_init(boot_info);
    unreachable();
}
