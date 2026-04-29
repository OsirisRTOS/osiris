__ram_start = ORIGIN(RAM);
__ram_end = ORIGIN(RAM) + LENGTH(RAM);
__stack_size = 0x8000;

#if OSIRIS_DEBUG_RUNTIMESYMBOLS
/* at least 250kb for the symbol table */
__syms_size = 0x3e800;
#else
/* if we don't want runtime symbols, we have exactly 4 null bytes which represent the size of the section as u32 */
__syms_size = 4;
#endif


/* 
We need to ensure that .data and .bss will be emitted in one segment.
The reason for that is, that with ropi-rwpi enabled LLD will generate incorrect r9 relative addressing when multiple segments are involved.
See https://github.com/llvm/llvm-project/issues/55628 for details.
*/
PHDRS
{
  text PT_LOAD FLAGS((1 << 2) | (1 << 0)); /* PF_X | PF_R */
  ro   PT_LOAD FLAGS((1 << 0));            /* PF_R        */
  data PT_LOAD FLAGS((1 << 1) | (1 << 0)); /* PF_W | PF_R */
}

SECTIONS
{
    /* Our vector table. */
    .ivt ORIGIN(FLASH) :
    {
        __vector_table = .;

        KEEP(*(.ivt.core));
        KEEP(*(.ivt.ext));
    } > FLASH :text

    .text :
    {
        *(.text .text.* .gnu.linkonce.t*)
        *(.gnu.warning)
    } > FLASH :text

    /* Some arm exception stuff */
    .ARM.extab : { *(.ARM.extab* .gnu.linkonce.armextab.*) } > FLASH :text
    . = ALIGN(4);
    PROVIDE_HIDDEN(__exidx_start = .);
    .ARM.exidx : { *(.ARM.exidx* .gnu.linkonce.armexidx.*) } > FLASH :text
    PROVIDE_HIDDEN(__exidx_end = .);

    .preinit_array :
    {
        PROVIDE_HIDDEN (__preinit_array_start = .);
        KEEP(*(.preinit_array*))
        PROVIDE_HIDDEN (__preinit_array_end = .);
    } > FLASH :text

    .init_array :
    {
        PROVIDE_HIDDEN (__init_array_start = .);
        KEEP (*(SORT(.init_array.*)))
        KEEP (*(.init_array*))
        PROVIDE_HIDDEN (__init_array_end = .);
    } > FLASH :text

    .fini_array :
    {
        PROVIDE_HIDDEN (__fini_array_start = .);
        KEEP (*(SORT(.fini_array.*)))
        KEEP (*(.fini_array*))
        PROVIDE_HIDDEN (__fini_array_end = .);
    } > FLASH :text

    .rodata :
    {
         *(.lit)
        *(.rodata .rodata.* .gnu.linkonce.r*)
        . = ALIGN(0x4);
        __rom_end = .;
    } > FLASH :ro

    /* A section of memory where the runtime symbols will be placed. 
     * We just make sure that nothing leaks into this memory space.
    */
    .syms_area :
    {
        . = ALIGN(4);
        __syms_area_start = .;
        KEEP(*(.syms_area))
        FILL(0x00);
        /* BYTE(0x00); make sure the section is not empty */
        . = . + __syms_size;
        __syms_area_end = .;
    } > FLASH :ro

    __data = LOADADDR(.bootinfo);
    .bootinfo : 
    {
        __data_start = .;
        KEEP(*(.bootinfo))
        . = ALIGN(4);
    } > RAM AT > FLASH :data

    .data :
    {
        *(.data .data.* .gnu.linkonce.d*)
        . = ALIGN(4);
        __data_end = .;
    } > RAM AT > FLASH :data

    .bss :
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > RAM :data

    .stack (NOLOAD) :
    {
        . = ALIGN(4);
        __stack_start = .;
        . = . + __stack_size;
        . = ALIGN(4);
        __stack_top = .;
    } > RAM

    /DISCARD/ : { *(.note.GNU-stack) *(.gnu_debuglink) *(.gnu.lto_*) }
}