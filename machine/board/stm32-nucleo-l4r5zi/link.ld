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
        KEEP(*(.ivt.ext));
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

    /DISCARD/ : { *(.note.GNU-stack) *(.gnu_debuglink) *(.gnu.lto_*) }
}

PROVIDE(wwdg_hndlr = default_hndlr);
PROVIDE(pvd_pvm_hndlr = default_hndlr);
PROVIDE(tamp_stamp_hndlr = default_hndlr);
PROVIDE(rtc_wkup_hndlr = default_hndlr);
PROVIDE(flash_hndlr = default_hndlr);
PROVIDE(rcc_hndlr = default_hndlr);
PROVIDE(exti0_hndlr = default_hndlr);
PROVIDE(exti1_hndlr = default_hndlr);
PROVIDE(exti2_hndlr = default_hndlr);
PROVIDE(exti3_hndlr = default_hndlr);
PROVIDE(exti4_hndlr = default_hndlr);
PROVIDE(dma1_ch1_hndlr = default_hndlr);
PROVIDE(dma1_ch2_hndlr = default_hndlr);
PROVIDE(dma1_ch3_hndlr = default_hndlr);
PROVIDE(dma1_ch4_hndlr = default_hndlr);
PROVIDE(dma1_ch5_hndlr = default_hndlr);
PROVIDE(dma1_ch6_hndlr = default_hndlr);
PROVIDE(dma1_ch7_hndlr = default_hndlr);
PROVIDE(adc1_hndlr = default_hndlr);
PROVIDE(can1_tx_hndlr = default_hndlr);
PROVIDE(can1_rx0_hndlr = default_hndlr);
PROVIDE(can1_rx1_hndlr = default_hndlr);
PROVIDE(can1_sce_hndlr = default_hndlr);
PROVIDE(exti9_5_hndlr = default_hndlr);
PROVIDE(tim1_brk_tim15_hndlr = default_hndlr);
PROVIDE(tim1_up_tim16_hndlr = default_hndlr);
PROVIDE(tim1_trg_com_tim17_hndlr = default_hndlr);
PROVIDE(tim1_cc_hndlr = default_hndlr);
PROVIDE(tim2_hndlr = default_hndlr);
PROVIDE(tim3_hndlr = default_hndlr);
PROVIDE(tim4_hndlr = default_hndlr);
PROVIDE(i2c1_ev_hndlr = default_hndlr);
PROVIDE(i2c1_er_hndlr = default_hndlr);
PROVIDE(i2c2_ev_hndlr = default_hndlr);
PROVIDE(i2c2_er_hndlr = default_hndlr);
PROVIDE(spi1_hndlr = default_hndlr);
PROVIDE(spi2_hndlr = default_hndlr);
PROVIDE(usart1_hndlr = default_hndlr);
PROVIDE(usart2_hndlr = default_hndlr);
PROVIDE(usart3_hndlr = default_hndlr);
PROVIDE(exti15_10_hndlr = default_hndlr);
PROVIDE(rtc_alarm_hndlr = default_hndlr);
PROVIDE(dfsdm1_flt3_hndlr = default_hndlr);
PROVIDE(tim8_brk_hndlr = default_hndlr);
PROVIDE(tim8_up_hndlr = default_hndlr);
PROVIDE(tim8_trg_com_hndlr = default_hndlr);
PROVIDE(tim8_cc_hndlr = default_hndlr);
PROVIDE(fmc_hndlr = default_hndlr);
PROVIDE(sdmmc1_hndlr = default_hndlr);
PROVIDE(tim5_hndlr = default_hndlr);
PROVIDE(spi3_hndlr = default_hndlr);
PROVIDE(uart4_hndlr = default_hndlr);
PROVIDE(uart5_hndlr = default_hndlr);
PROVIDE(tim6_dac_under_hndlr = default_hndlr);
PROVIDE(tim7_hndlr = default_hndlr);
PROVIDE(dma2_ch1_hndlr = default_hndlr);
PROVIDE(dma2_ch2_hndlr = default_hndlr);
PROVIDE(dma2_ch3_hndlr = default_hndlr);
PROVIDE(dma2_ch4_hndlr = default_hndlr);
PROVIDE(dma2_ch5_hndlr = default_hndlr);
PROVIDE(dfsdm1_flt0_hndlr = default_hndlr);
PROVIDE(dfsdm1_flt1_hndlr = default_hndlr);
PROVIDE(dfsdm1_flt2_hndlr = default_hndlr);
PROVIDE(comp_hndlr = default_hndlr);
PROVIDE(lptim1_hndlr = default_hndlr);
PROVIDE(lptim2_hndlr = default_hndlr);
PROVIDE(otg_fs_hndlr = default_hndlr);
PROVIDE(dma2_ch6_hndlr = default_hndlr);
PROVIDE(dma2_ch7_hndlr = default_hndlr);
PROVIDE(lpuart1_hndlr = default_hndlr);
PROVIDE(octospi1_hndlr = default_hndlr);
PROVIDE(i2c3_ev_hndlr = default_hndlr);
PROVIDE(i2c3_er_hndlr = default_hndlr);
PROVIDE(sai1_hndlr = default_hndlr);
PROVIDE(sai2_hndlr = default_hndlr);
PROVIDE(octospi2_hndlr = default_hndlr);
PROVIDE(tsc_hndlr = default_hndlr);
PROVIDE(dsihot_hndlr = default_hndlr);
PROVIDE(aes_hndlr = default_hndlr);
PROVIDE(rng_hndlr = default_hndlr);
PROVIDE(fpu_hndlr = default_hndlr);
PROVIDE(hash_crs_hndlr = default_hndlr);
PROVIDE(i2c4_ev_hndlr = default_hndlr);
PROVIDE(i2c4_er_hndlr = default_hndlr);
PROVIDE(dcmi_hndlr = default_hndlr);
PROVIDE(dma2d_hndlr = default_hndlr);
PROVIDE(lcd_tft_hndlr = default_hndlr);
PROVIDE(lcd_tft_er_hndlr = default_hndlr);
PROVIDE(gfxmmu_hndlr = default_hndlr);
PROVIDE(dmamux1_ovr_hndlr = default_hndlr);