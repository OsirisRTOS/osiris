SEARCH_DIR(.)
__DYNAMIC = 0;

ENTRY(reset_hndlr)

MEMORY
{
  FLASH (rx) : ORIGIN = 0x08000000, LENGTH = 2M
  RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 640K
}

__stack_top = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS
{
    .ivt ORIGIN(FLASH) :
    {
        __vector_table = .;

        KEEP(*(.ivt.core));
        KEEP(*(.ivt.device));
    } > FLASH

    .text :
    {
        *(.text .text.* .gnu.linkonce.t*)
        *(.gnu.warning)
    } > FLASH

    .ARM.extab : { *(.ARM.extab* .gnu.linkonce.armextab.*) } > FLASH
    . = ALIGN(4);
    PROVIDE_HIDDEN(__exidx_start = .);
    .ARM.exidx : { *(.ARM.exidx* .gnu.linkonce.armexidx.*) } > FLASH
    PROVIDE_HIDDEN(__exidx_end = .);

    .preinit_array :
    {
        PROVIDE_HIDDEN (__preinit_array_start = .);
        KEEP(*(.preinit_array*))
        PROVIDE_HIDDEN (__preinit_array_end = .);
    } > FLASH

    .init_array :
    {
        PROVIDE_HIDDEN (__init_array_start = .);
        KEEP (*(SORT(.init_array.*)))
        KEEP (*(.init_array*))
        PROVIDE_HIDDEN (__init_array_end = .);
    } > FLASH

    .fini_array :
    {
        PROVIDE_HIDDEN (__fini_array_start = .);
        KEEP (*(SORT(.fini_array.*)))
        KEEP (*(.fini_array*))
        PROVIDE_HIDDEN (__fini_array_end = .);
    } > FLASH

    .rodata :
    {
         *(.lit)
        *(.rodata .rodata.* .gnu.linkonce.r*)
        . = ALIGN(0x4);
        __rom_end = .;
    } > FLASH

    __data = LOADADDR(.data);
    .data :
    {
        __data_start = .;
        *(.data .data.* .gnu.linkonce.d*)
        . = ALIGN(4);
        __data_end = .;
    } > RAM AT > FLASH

    .bss (NOLOAD):
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > RAM

    . = ALIGN(4);
    __heap_start = .;
}