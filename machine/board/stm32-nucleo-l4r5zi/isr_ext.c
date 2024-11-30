
#include <stdint.h>

/*
 * External interrupt vector table for the STM32 Nucleo L4R5ZI
 * Used references: https://www.st.com/resource/en/reference_manual/rm0432-stm32l4-series-advanced-armbased-32bit-mcus-stmicroelectronics.pdf
 */

extern void wwdg_hndlr(void);
extern void pvd_pvm_hndlr(void);
extern void tamp_stamp_hndlr(void);
extern void rtc_wkup_hndlr(void);
extern void flash_hndlr(void);
extern void rcc_hndlr(void);
extern void exti0_hndlr(void);
extern void exti1_hndlr(void);
extern void exti2_hndlr(void);
extern void exti3_hndlr(void);
extern void exti4_hndlr(void);
extern void dma1_ch1_hndlr(void);
extern void dma1_ch2_hndlr(void);
extern void dma1_ch3_hndlr(void);
extern void dma1_ch4_hndlr(void);
extern void dma1_ch5_hndlr(void);
extern void dma1_ch6_hndlr(void);
extern void dma1_ch7_hndlr(void);
extern void adc1_hndlr(void);
extern void can1_tx_hndlr(void);
extern void can1_rx0_hndlr(void);
extern void can1_rx1_hndlr(void);
extern void can1_sce_hndlr(void);
extern void exti9_5_hndlr(void);
extern void tim1_brk_tim15_hndlr(void);
extern void tim1_up_tim16_hndlr(void);
extern void tim1_trg_com_tim17_hndlr(void);
extern void tim1_cc_hndlr(void);
extern void tim2_hndlr(void);
extern void tim3_hndlr(void);
extern void tim4_hndlr(void);
extern void i2c1_ev_hndlr(void);
extern void i2c1_er_hndlr(void);
extern void i2c2_ev_hndlr(void);
extern void i2c2_er_hndlr(void);
extern void spi1_hndlr(void);
extern void spi2_hndlr(void);
extern void usart1_hndlr(void);
extern void usart2_hndlr(void);
extern void usart3_hndlr(void);
extern void exti15_10_hndlr(void);
extern void rtc_alarm_hndlr(void);
extern void dfsdm1_flt3_hndlr(void);
extern void tim8_brk_hndlr(void);
extern void tim8_up_hndlr(void);
extern void tim8_trg_com_hndlr(void);
extern void tim8_cc_hndlr(void);
extern void fmc_hndlr(void);
extern void sdmmc1_hndlr(void);
extern void tim5_hndlr(void);
extern void spi3_hndlr(void);
extern void uart4_hndlr(void);
extern void uart5_hndlr(void);
extern void tim6_dac_under_hndlr(void);
extern void tim7_hndlr(void);
extern void dma2_ch1_hndlr(void);
extern void dma2_ch2_hndlr(void);
extern void dma2_ch3_hndlr(void);
extern void dma2_ch4_hndlr(void);
extern void dma2_ch5_hndlr(void);
extern void dfsdm1_flt0_hndlr(void);
extern void dfsdm1_flt1_hndlr(void);
extern void dfsdm1_flt2_hndlr(void);
extern void comp_hndlr(void);
extern void lptim1_hndlr(void);
extern void lptim2_hndlr(void);
extern void otg_fs_hndlr(void);
extern void dma2_ch6_hndlr(void);
extern void dma2_ch7_hndlr(void);
extern void lpuart1_hndlr(void);
extern void octospi1_hndlr(void);
extern void i2c3_ev_hndlr(void);
extern void i2c3_er_hndlr(void);
extern void sai1_hndlr(void);
extern void sai2_hndlr(void);
extern void octospi2_hndlr(void);
extern void tsc_hndlr(void);
extern void dsihot_hndlr(void);
extern void aes_hndlr(void);
extern void rng_hndlr(void);
extern void fpu_hndlr(void);
extern void hash_crs_hndlr(void);
extern void i2c4_er_hndlr(void);
extern void i2c4_ev_hndlr(void);
extern void dcmi_hndlr(void);
extern void dma2d_hndlr(void);
extern void lcd_tft_hndlr(void);
extern void lcd_tft_er_hndlr(void);
extern void gfxmmu_hndlr(void);
extern void dmamux1_ovr_hndlr(void);

const uintptr_t vector_table_ext[] __attribute__((section(".ivt.ext"))) = {
    (uintptr_t)&wwdg_hndlr,
    (uintptr_t)&pvd_pvm_hndlr,
    (uintptr_t)&tamp_stamp_hndlr,
    (uintptr_t)&rtc_wkup_hndlr,
    (uintptr_t)&flash_hndlr,
    (uintptr_t)&rcc_hndlr,
    (uintptr_t)&exti0_hndlr,
    (uintptr_t)&exti1_hndlr,
    (uintptr_t)&exti2_hndlr,
    (uintptr_t)&exti3_hndlr,
    (uintptr_t)&exti4_hndlr,
    (uintptr_t)&dma1_ch1_hndlr,
    (uintptr_t)&dma1_ch2_hndlr,
    (uintptr_t)&dma1_ch3_hndlr,
    (uintptr_t)&dma1_ch4_hndlr,
    (uintptr_t)&dma1_ch5_hndlr,
    (uintptr_t)&dma1_ch6_hndlr,
    (uintptr_t)&dma1_ch7_hndlr,
    (uintptr_t)&adc1_hndlr,
    (uintptr_t)&can1_tx_hndlr,
    (uintptr_t)&can1_rx0_hndlr,
    (uintptr_t)&can1_rx1_hndlr,
    (uintptr_t)&can1_sce_hndlr,
    (uintptr_t)&exti9_5_hndlr,
    (uintptr_t)&tim1_brk_tim15_hndlr,
    (uintptr_t)&tim1_up_tim16_hndlr,
    (uintptr_t)&tim1_trg_com_tim17_hndlr,
    (uintptr_t)&tim1_cc_hndlr,
    (uintptr_t)&tim2_hndlr,
    (uintptr_t)&tim3_hndlr,
    (uintptr_t)&tim4_hndlr,
    (uintptr_t)&i2c1_ev_hndlr,
    (uintptr_t)&i2c1_er_hndlr,
    (uintptr_t)&i2c2_ev_hndlr,
    (uintptr_t)&i2c2_er_hndlr,
    (uintptr_t)&spi1_hndlr,
    (uintptr_t)&spi2_hndlr,
    (uintptr_t)&usart1_hndlr,
    (uintptr_t)&usart2_hndlr,
    (uintptr_t)&usart3_hndlr,
    (uintptr_t)&exti15_10_hndlr,
    (uintptr_t)&rtc_alarm_hndlr,
    (uintptr_t)&dfsdm1_flt3_hndlr,
    (uintptr_t)&tim8_brk_hndlr,
    (uintptr_t)&tim8_up_hndlr,
    (uintptr_t)&tim8_trg_com_hndlr,
    (uintptr_t)&tim8_cc_hndlr,
    0,
    (uintptr_t)&fmc_hndlr,
    (uintptr_t)&sdmmc1_hndlr,
    (uintptr_t)&tim5_hndlr,
    (uintptr_t)&spi3_hndlr,
    (uintptr_t)&uart4_hndlr,
    (uintptr_t)&uart5_hndlr,
    (uintptr_t)&tim6_dac_under_hndlr,
    (uintptr_t)&tim7_hndlr,
    (uintptr_t)&dma2_ch1_hndlr,
    (uintptr_t)&dma2_ch2_hndlr,
    (uintptr_t)&dma2_ch3_hndlr,
    (uintptr_t)&dma2_ch4_hndlr,
    (uintptr_t)&dma2_ch5_hndlr,
    (uintptr_t)&dfsdm1_flt0_hndlr,
    (uintptr_t)&dfsdm1_flt1_hndlr,
    (uintptr_t)&dfsdm1_flt2_hndlr,
    (uintptr_t)&comp_hndlr,
    (uintptr_t)&lptim1_hndlr,
    (uintptr_t)&lptim2_hndlr,
    (uintptr_t)&otg_fs_hndlr,
    (uintptr_t)&dma2_ch6_hndlr,
    (uintptr_t)&dma2_ch7_hndlr,
    (uintptr_t)&lpuart1_hndlr,
    (uintptr_t)&octospi1_hndlr,
    (uintptr_t)&i2c3_ev_hndlr,
    (uintptr_t)&i2c3_er_hndlr,
    (uintptr_t)&sai1_hndlr,
    (uintptr_t)&sai2_hndlr,
    (uintptr_t)&octospi2_hndlr,
    (uintptr_t)&tsc_hndlr,
    (uintptr_t)&dsihot_hndlr,
    (uintptr_t)&aes_hndlr,
    (uintptr_t)&rng_hndlr,
    (uintptr_t)&fpu_hndlr,
    (uintptr_t)&hash_crs_hndlr,
    (uintptr_t)&i2c4_er_hndlr,
    (uintptr_t)&i2c4_ev_hndlr,
    (uintptr_t)&dcmi_hndlr,
    0,
    0,
    0,
    0,
    (uintptr_t)&dma2d_hndlr,
    (uintptr_t)&lcd_tft_hndlr,
    (uintptr_t)&lcd_tft_er_hndlr,
    (uintptr_t)&gfxmmu_hndlr,
    (uintptr_t)&dmamux1_ovr_hndlr};