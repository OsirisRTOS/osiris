ENTRY(_start)

#ifndef MCU_HEADER
#error "MCU_HEADER is not defined."
#endif

#include MCU_HEADER

__ram_start = ORIGIN(RAM);
__ram_end = ORIGIN(RAM) + LENGTH(RAM);

#ifdef CONFIG_RUNTIME_SYMBOLS
/* at least 250kb for the symbol table */
__syms_size = 0x3e800;
#else
/* if we don't want runtime symbols, we have exactly 4 null bytes which represent the size of the section as u32 */
__syms_size = 4;
#endif

SECTIONS
{
    /* Our vector table. */
    .ivt ORIGIN(FLASH) :
    {
        __vector_table = .;

        KEEP(*(.ivt.core));
        KEEP(*(.ivt.ext));
    } > FLASH

    .stack (NOLOAD) :
    {
        . = ALIGN(4);
        __stack_start = .;
        . = . + __stack_size;
        . = ALIGN(4);
        __stack_top = .;
    } > RAM

    .text :
    {
        *(.text .text.* .gnu.linkonce.t*)
        *(.gnu.warning)
    } > FLASH

    /* Some arm exception stuff */
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
        . = ALIGN(4);
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(4);
        __bss_end = .;
    } > RAM

    /* A section of memory where the runtime symbols will be placed. 
     * We just make sure that nothing leaks into this memory space.
    */
    .syms_area :
    {
        . = ALIGN(4);
        __syms_area_start = .;
        KEEP(*(.syms_area))
        BYTE(0x00); /* make sure the section is not empty */
        . += __syms_size;
        __syms_area_end = .;
    } > FLASH

    /DISCARD/ : { *(.note.GNU-stack) *(.gnu_debuglink) *(.gnu.lto_*) }
}

PROVIDE(irq_hndlr = default_hndlr);

PROVIDE(wwdg_hndlr = irq_enter_no_switch_fp);
PROVIDE(pvd_pvm_hndlr = irq_enter_no_switch_fp);
PROVIDE(tamp_stamp_hndlr = irq_enter_no_switch_fp);
PROVIDE(rtc_wkup_hndlr = irq_enter_no_switch_fp);
PROVIDE(flash_hndlr = irq_enter_no_switch_fp);
PROVIDE(rcc_hndlr = irq_enter_no_switch_fp);
PROVIDE(exti0_hndlr = irq_enter_no_switch_fp);
PROVIDE(exti1_hndlr = irq_enter_no_switch_fp);
PROVIDE(exti2_hndlr = irq_enter_no_switch_fp);
PROVIDE(exti3_hndlr = irq_enter_no_switch_fp);
PROVIDE(exti4_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma1_ch1_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma1_ch2_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma1_ch3_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma1_ch4_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma1_ch5_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma1_ch6_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma1_ch7_hndlr = irq_enter_no_switch_fp);
PROVIDE(adc1_hndlr = irq_enter_no_switch_fp);
PROVIDE(can1_tx_hndlr = irq_enter_no_switch_fp);
PROVIDE(can1_rx0_hndlr = irq_enter_no_switch_fp);
PROVIDE(can1_rx1_hndlr = irq_enter_no_switch_fp);
PROVIDE(can1_sce_hndlr = irq_enter_no_switch_fp);
PROVIDE(exti9_5_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim1_brk_tim15_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim1_up_tim16_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim1_trg_com_tim17_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim1_cc_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim2_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim3_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim4_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c1_ev_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c1_er_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c2_ev_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c2_er_hndlr = irq_enter_no_switch_fp);
PROVIDE(spi1_hndlr = irq_enter_no_switch_fp);
PROVIDE(spi2_hndlr = irq_enter_no_switch_fp);
PROVIDE(usart1_hndlr = irq_enter_no_switch_fp);
PROVIDE(usart2_hndlr = irq_enter_no_switch_fp);
PROVIDE(usart3_hndlr = irq_enter_no_switch_fp);
PROVIDE(exti15_10_hndlr = irq_enter_no_switch_fp);
PROVIDE(rtc_alarm_hndlr = irq_enter_no_switch_fp);
PROVIDE(dfsdm1_flt3_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim8_brk_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim8_up_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim8_trg_com_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim8_cc_hndlr = irq_enter_no_switch_fp);
PROVIDE(fmc_hndlr = irq_enter_no_switch_fp);
PROVIDE(sdmmc1_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim5_hndlr = irq_enter_no_switch_fp);
PROVIDE(spi3_hndlr = irq_enter_no_switch_fp);
PROVIDE(uart4_hndlr = irq_enter_no_switch_fp);
PROVIDE(uart5_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim6_dac_under_hndlr = irq_enter_no_switch_fp);
PROVIDE(tim7_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2_ch1_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2_ch2_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2_ch3_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2_ch4_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2_ch5_hndlr = irq_enter_no_switch_fp);
PROVIDE(dfsdm1_flt0_hndlr = irq_enter_no_switch_fp);
PROVIDE(dfsdm1_flt1_hndlr = irq_enter_no_switch_fp);
PROVIDE(dfsdm1_flt2_hndlr = irq_enter_no_switch_fp);
PROVIDE(comp_hndlr = irq_enter_no_switch_fp);
PROVIDE(lptim1_hndlr = irq_enter_no_switch_fp);
PROVIDE(lptim2_hndlr = irq_enter_no_switch_fp);
PROVIDE(otg_fs_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2_ch6_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2_ch7_hndlr = irq_enter_no_switch_fp);
PROVIDE(lpuart1_hndlr = irq_enter_no_switch_fp);
PROVIDE(octospi1_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c3_ev_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c3_er_hndlr = irq_enter_no_switch_fp);
PROVIDE(sai1_hndlr = irq_enter_no_switch_fp);
PROVIDE(sai2_hndlr = irq_enter_no_switch_fp);
PROVIDE(octospi2_hndlr = irq_enter_no_switch_fp);
PROVIDE(tsc_hndlr = irq_enter_no_switch_fp);
PROVIDE(dsihot_hndlr = irq_enter_no_switch_fp);
PROVIDE(aes_hndlr = irq_enter_no_switch_fp);
PROVIDE(rng_hndlr = irq_enter_no_switch_fp);
PROVIDE(fpu_hndlr = irq_enter_no_switch_fp);
PROVIDE(hash_crs_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c4_ev_hndlr = irq_enter_no_switch_fp);
PROVIDE(i2c4_er_hndlr = irq_enter_no_switch_fp);
PROVIDE(dcmi_hndlr = irq_enter_no_switch_fp);
PROVIDE(dma2d_hndlr = irq_enter_no_switch_fp);
PROVIDE(lcd_tft_hndlr = irq_enter_no_switch_fp);
PROVIDE(lcd_tft_er_hndlr = irq_enter_no_switch_fp);
PROVIDE(gfxmmu_hndlr = irq_enter_no_switch_fp);
PROVIDE(dmamux1_ovr_hndlr = irq_enter_no_switch_fp);