#include "lib.h"
#include <stdatomic.h>
#include <assert.h>
#include <stm32l4xx_hal.h>
#include "stm32l4xx_hal_def.h"
#include "stm32l4xx_hal_rcc.h"
#include "stm32l4xx_hal_rcc_ex.h"
#include <stm32l4xx_ll_tim.h>

static volatile uint64_t monotonic_hi = 0;
static volatile uint32_t tick = 0;

uint8_t RTC_BKP_IDX = 31; // backup register used to store magic value indicating RTC has been set
uint32_t RTC_BKP_MAGIC = 0x4F534952U;

const int CONST_RCC_IRQn = RCC_IRQn;

// use msb for the error type
// lower byte(s) contain hal status
enum ErrorTypes : uint64_t {
     ERROR_CONTROL_VOLTAGE_SCALING = 0x01U << 56U,
     ERROR_RCC_OSC_CONFIG = 0x02U << 56U,
     ERROR_RCC_CLOCK_CONFIG = 0x03U << 56U,
     ERROR_RTC_INIT_CLOCK_SOURCE = 0x04U << 56U,
     ERROR_RTC_INIT = 0x05U << 56U,
     ERROR_RTC_GET_TIME = 0x06U << 56U,
     ERROR_RTC_GET_DATE = 0x07U << 56U,
     ERROR_RTC_SET_TIME = 0x08U << 56U,
     ERROR_RTC_SET_DATE = 0x09U << 56U
};

static RTC_HandleTypeDef rtc_handle;

/**
* Try to use LSE, fall back to LSI and enable CSS if both are available.
* @retval HAL_StatusTypeDef codes:
* bit 0-1: selecting LSE clock source
* bit 8-9: selecting LSI clock source or waiting for LSI clock source to become ready
 */
static int init_rtc_clock_source(void)
{
    // TODO check if we can use HAL_RCC_OscConfig instead of doing the same things manually
    HAL_PWR_EnableBkUpAccess();

    __HAL_RCC_LSI_ENABLE();

    __HAL_RCC_LSEDRIVE_CONFIG(RCC_LSEDRIVE_HIGH);
    __HAL_RCC_LSE_CONFIG(RCC_LSE_ON);

    RCC_PeriphCLKInitTypeDef periph = {0};
    periph.PeriphClockSelection = RCC_PERIPHCLK_RTC;

    periph.RTCClockSelection = RCC_RTCCLKSOURCE_LSE;
    HAL_StatusTypeDef lse_status = HAL_RCCEx_PeriphCLKConfig(&periph);

    HAL_StatusTypeDef lsi_status = 0;

    if (lse_status != HAL_OK) {
      // fallback to LSI
      periph.RTCClockSelection = RCC_RTCCLKSOURCE_LSI;
      lsi_status = HAL_RCCEx_PeriphCLKConfig(&periph);
      // if LSI selection also fails, return both errors
      if (lsi_status != HAL_OK) {
        return lse_status | (lsi_status << 8);
      }
    }

    // ensure LSI is ready
    uint32_t tickstart = HAL_GetTick();
    while (__HAL_RCC_GET_FLAG(RCC_FLAG_LSIRDY) == RESET) {
      if ((HAL_GetTick() - tickstart) > RCC_LSE_TIMEOUT_VALUE) {
        lsi_status = HAL_TIMEOUT;
        break;
      }
    }

    __HAL_RCC_RTC_ENABLE();

    // clock security system requires both LSE and LSI to be enabled.
    if (lse_status == HAL_OK && lsi_status == HAL_OK) {
       HAL_RCCEx_EnableLSECSS();
       __HAL_RCC_ENABLE_IT(RCC_IT_LSECSS);
    }
    HAL_PWR_DisableBkUpAccess();

    return lse_status | (lsi_status << 8);
}

_Bool irq_is_css(void) { 
    return __HAL_RCC_GET_IT(RCC_IT_CSS);
}

_Bool irq_is_lse_css(void)  { 
    return __HAL_RCC_GET_IT(RCC_IT_LSECSS);
}

uint64_t update_clock_cfg(void);
HAL_StatusTypeDef switch_to_hsi(void);
void css_hndlr() {
    __HAL_RCC_CLEAR_IT(RCC_IT_CSS);

    switch_to_hsi();
    update_clock_cfg();
}

void css_lse_hndlr()
{
    __HAL_RCC_CLEAR_IT(RCC_IT_LSECSS);
    HAL_PWR_EnableBkUpAccess();

    // The software MUST then disable the LSECSSON bit
    HAL_RCCEx_DisableLSECSS();
    // stop the defective 32 kHz oscillator (disabling LSEON)
    __HAL_RCC_LSE_CONFIG(RCC_LSE_OFF);

    // and change the RTC clock source (no clock or LSI or HSE, with RTCSEL)
    RCC_PeriphCLKInitTypeDef periph = {0};
    periph.PeriphClockSelection = RCC_PERIPHCLK_RTC;
    periph.RTCClockSelection = RCC_RTCCLKSOURCE_LSI;
    HAL_StatusTypeDef status = HAL_RCCEx_PeriphCLKConfig(&periph);
    if (status != HAL_OK) {
        // internal clock failed, try again later?
        __HAL_RCC_RTC_DISABLE();
    }
    HAL_PWR_DisableBkUpAccess();
}

uint32_t get_rtc_backup_register(uint8_t index);
uint64_t set_rtc_raw(uint64_t raw);
uint64_t init_rtc(void)
{
    __HAL_RCC_PWR_CLK_ENABLE();

    // one source failing is not a problem if the other works
    int clock_source = init_rtc_clock_source();
    if (clock_source & 0xFF != HAL_OK && clock_source & 0xFF00 != HAL_OK) {
        return ERROR_RTC_INIT_CLOCK_SOURCE | clock_source;
    }

    rtc_handle.Instance = RTC;
    rtc_handle.Init.HourFormat = RTC_HOURFORMAT_24;
    rtc_handle.Init.AsynchPrediv = 0x7FU;
    rtc_handle.Init.SynchPrediv = 0x00FFU;
    rtc_handle.Init.OutPut = RTC_OUTPUT_DISABLE;
    rtc_handle.Init.OutPutRemap = RTC_OUTPUT_REMAP_NONE;
    rtc_handle.Init.OutPutPolarity = RTC_OUTPUT_POLARITY_HIGH;
    rtc_handle.Init.OutPutType = RTC_OUTPUT_TYPE_OPENDRAIN;

    int ret = HAL_RTC_Init(&rtc_handle);
    if (ret != HAL_OK) {
        return ERROR_RTC_INIT | ret;
    }

    if (get_rtc_backup_register(RTC_BKP_IDX) != RTC_BKP_MAGIC) {
        // Sat 01.01.2000
        unsigned long long time = ((uint64_t)0) |
           ((uint64_t)0 << 8U) |
           ((uint64_t)0 << 16U) |
           ((uint64_t)RTC_WEEKDAY_SATURDAY << 32U) |              
           ((uint64_t)RTC_MONTH_JANUARY << 40U) |
           ((uint64_t)1 << 48U) |
           ((uint64_t)0 << 56U);
        ret = set_rtc_raw(time);
        if (ret != 0) {
            return ret;
        }
    }
    return clock_source;
}

static void init_monotonic_timer(void)
{
    const uint32_t target_hz = 1000000U;
    uint32_t tim_clk = HAL_RCC_GetPCLK1Freq();

    monotonic_hi = 0;

    // If APB1 prescaler is not 1, timer clocks run at 2x PCLK1.
    if ((RCC->CFGR & RCC_CFGR_PPRE1) != RCC_CFGR_PPRE1_DIV1) {
        tim_clk *= 2U;
    }

    const uint32_t prescaler = (tim_clk / target_hz) - 1U;

    __HAL_RCC_TIM2_CLK_ENABLE();
    __HAL_RCC_TIM2_FORCE_RESET();
    __HAL_RCC_TIM2_RELEASE_RESET();

    HAL_NVIC_DisableIRQ(TIM2_IRQn);
    NVIC_ClearPendingIRQ(TIM2_IRQn);

    // URS ensures update flags/interrupts are only from real overflows.
    TIM2->CR1 = TIM_CR1_URS;
    TIM2->PSC = prescaler;
    TIM2->ARR = 0xFFFFFFFFU;
    TIM2->CNT = 0;
    TIM2->EGR = TIM_EGR_UG;

    // Clear pending flags and enable update interrupt for wrap extension.
    TIM2->SR = 0;
    TIM2->DIER = TIM_DIER_UIE;

    HAL_NVIC_SetPriority(TIM2_IRQn, 15, 0);
    HAL_NVIC_EnableIRQ(TIM2_IRQn);

    TIM2->CR1 |= TIM_CR1_CEN;

    // Clear any latent startup update state before first read.
    TIM2->SR = 0;
    NVIC_ClearPendingIRQ(TIM2_IRQn);
}

void tim2_hndlr(void)
{
    if (LL_TIM_IsActiveFlag_UPDATE(TIM2)) {
        LL_TIM_ClearFlag_UPDATE(TIM2);
        monotonic_hi += (1ULL << 32);
    }
}

HAL_StatusTypeDef switch_to_hsi() {
  RCC_OscInitTypeDef RCC_OscInitStruct = {0};
  RCC_OscInitStruct.OscillatorType = RCC_OSCILLATORTYPE_HSE;
  RCC_OscInitStruct.HSEState = RCC_HSE_OFF;
  RCC_OscInitStruct.PLL.PLLSource = RCC_PLLSOURCE_HSI;
  RCC_OscInitStruct.PLL.PLLN = 10;
  return HAL_RCC_OscConfig(&RCC_OscInitStruct);
}

uint64_t update_clock_cfg(void)
{
    RCC_ClkInitTypeDef RCC_ClkInitStruct = {0};

    RCC_ClkInitStruct.ClockType =
        RCC_CLOCKTYPE_SYSCLK |
        RCC_CLOCKTYPE_HCLK   |
        RCC_CLOCKTYPE_PCLK1  |
        RCC_CLOCKTYPE_PCLK2;

    RCC_ClkInitStruct.SYSCLKSource   = RCC_SYSCLKSOURCE_PLLCLK;
    RCC_ClkInitStruct.AHBCLKDivider  = RCC_SYSCLK_DIV1;
    RCC_ClkInitStruct.APB1CLKDivider = RCC_HCLK_DIV1;
    RCC_ClkInitStruct.APB2CLKDivider = RCC_HCLK_DIV1;

    int ret = HAL_RCC_ClockConfig(&RCC_ClkInitStruct, FLASH_LATENCY_4);
    if (ret != HAL_OK) {
       return ERROR_RCC_CLOCK_CONFIG | ret;
    }

    SystemCoreClockUpdate();
    init_monotonic_timer();
    return 0;
}

/*
setup hse
setup hsi
enable hse
hse works?
y: enable css
n: disable hse, enable hsi
setup sysclock
*/
uint64_t init_clock_cfg(void)
{
    RCC_OscInitTypeDef RCC_OscInitStruct = {0};

    /* 80 MHz on STM32L4+ => Range 1 normal mode, not boost */
    __HAL_RCC_PWR_CLK_ENABLE();

    int ret = HAL_PWREx_ControlVoltageScaling(PWR_REGULATOR_VOLTAGE_SCALE1);
    if (ret != HAL_OK) {
       return ERROR_CONTROL_VOLTAGE_SCALING | ret;
    }

    /* HSE 8Mhz -> PLL -> 80 MHz SYSCLK */
    RCC_OscInitStruct.OscillatorType = RCC_OSCILLATORTYPE_HSE | RCC_OSCILLATORTYPE_HSI;
    RCC_OscInitStruct.HSEState = RCC_HSE_ON;
    /* HSI16 -> PLL -> 80 MHz SYSCLK */
    RCC_OscInitStruct.HSIState = RCC_HSI_ON;
    RCC_OscInitStruct.HSICalibrationValue = RCC_HSICALIBRATION_DEFAULT;

    RCC_OscInitStruct.PLL.PLLState = RCC_PLL_ON;
    RCC_OscInitStruct.PLL.PLLSource = RCC_PLLSOURCE_HSE;
    RCC_OscInitStruct.PLL.PLLM = 1;
    RCC_OscInitStruct.PLL.PLLN = 20;
    RCC_OscInitStruct.PLL.PLLR = RCC_PLLR_DIV2;
    RCC_OscInitStruct.PLL.PLLP = RCC_PLLP_DIV7;   // arbitrary unless you use PLLP
    RCC_OscInitStruct.PLL.PLLQ = RCC_PLLQ_DIV2;   // arbitrary unless you use PLLQ

    // HSE failed, use HSI
    ret = HAL_RCC_OscConfig(&RCC_OscInitStruct);
    if (ret != HAL_OK) {
        ret = switch_to_hsi();
        if (ret != HAL_OK) {
            return ERROR_RCC_OSC_CONFIG | ret;
        }
    } else {
        HAL_RCC_EnableCSS();
    }
    return update_clock_cfg();
}

unsigned long long monotonic_now(void)
{
    uint64_t hi_1;
    uint64_t hi_2;
    uint32_t lo;
    uint32_t sr;

    // Retry if the overflow IRQ updates the high word while sampling.
    do {
        hi_1 = monotonic_hi;
        lo = TIM2->CNT;
        sr = TIM2->SR;
        hi_2 = monotonic_hi;
    } while (hi_1 != hi_2);

    // If overflow is pending but IRQ has not run yet, include that wrap.
    if ((sr & TIM_SR_UIF) != 0U && lo < 0x80000000U) {
        hi_1 += (1ULL << 32);
    }

    return hi_1 | (uint64_t)lo;
}

unsigned long long monotonic_freq(void)
{
    return 1000000ULL;
}

void delay_us(uint32_t delay_us)
{
    if (delay_us == 0U) {
        return;
    }

    uint64_t freq = monotonic_freq();
    if (freq == 0U) {
        return;
    }

    uint64_t ticks = (((uint64_t)delay_us * freq) + 999999ULL) / 1000000ULL;
    if (ticks == 0U) {
        ticks = 1U;
    }

    uint64_t start = monotonic_now();
    while ((monotonic_now() - start) < ticks) {
    }
}

// Use like: (uint32_t)(HAL_GetTick() - start) >= Timeout
// Not: HAL_GetTick() > start + Timeout
// The first version handles wrapping on overflow correctly, while the second does not.
uint32_t HAL_GetTick(void)
{
    return tick;
}

void do_tick(void)
{
    tick++;
}


uint32_t get_rtc_backup_register(uint8_t index)
{
    return HAL_RTCEx_BKUPRead(&rtc_handle, RTC_BKP_DR0 + index);
}

void set_rtc_backup_register(uint8_t index, uint32_t value)
{
    if (index == 32) {
        // disable external crystal
        HAL_PWR_EnableBkUpAccess();
        RCC->CR |= RCC_CR_HSEBYP;   // Set bypass mode
        RCC->CR |= RCC_CR_HSEON;    // Re-enable
        HAL_PWR_DisableBkUpAccess();
        return;
    }
    HAL_PWR_EnableBkUpAccess();
    HAL_RTCEx_BKUPWrite(&rtc_handle, RTC_BKP_DR0 + index, value);
    HAL_PWR_DisableBkUpAccess();
}

uint64_t rtc_raw(void)
{
    RTC_TimeTypeDef time = {0};
    RTC_DateTypeDef date = {0};

    int ret = HAL_RTC_GetTime(&rtc_handle, &time, RTC_FORMAT_BCD);
    if (ret != HAL_OK) {
        return ERROR_RTC_GET_TIME | ret;
    }

    ret = HAL_RTC_GetDate(&rtc_handle, &date, RTC_FORMAT_BCD);
    if (ret != HAL_OK) {
        return ERROR_RTC_GET_DATE | ret;
    }

    return ((uint64_t)time.Hours) |
           ((uint64_t)time.Minutes << 8U) |
           ((uint64_t)time.Seconds << 16U) |
           ((uint64_t)date.WeekDay << 24U) |              
           ((uint64_t)date.Month << 32U) |
           ((uint64_t)date.Date << 40U) |
           ((uint64_t)date.Year << 48U);
}

uint64_t set_rtc_raw(uint64_t raw)
{
    RTC_TimeTypeDef rtc_time = {0};
    RTC_DateTypeDef rtc_date = {0};

    rtc_time.Hours = (uint8_t)(raw & 0xFFU);
    rtc_time.Minutes = (uint8_t)((raw >> 8U) & 0xFFU);
    rtc_time.Seconds = (uint8_t)((raw >> 16U) & 0xFFU);
    rtc_time.TimeFormat = RTC_HOURFORMAT_24;

    rtc_date.WeekDay = (uint8_t)((raw >> 32U) & 0xFFU);
    rtc_date.Month = (uint8_t)((raw >> 40U) & 0xFFU);
    rtc_date.Date = (uint8_t)((raw >> 48U) & 0xFFU);
    rtc_date.Year = (uint8_t)((raw >> 56U) & 0xFFU);

    int ret = HAL_RTC_SetTime(&rtc_handle, &rtc_time, RTC_FORMAT_BCD);
    if (ret != HAL_OK) {
        return ERROR_RTC_SET_TIME | ret;
    }

    ret = HAL_RTC_SetDate(&rtc_handle, &rtc_date, RTC_FORMAT_BCD);
    if (ret != HAL_OK) {
        return ERROR_RTC_SET_DATE | ret;
    }

    set_rtc_backup_register(RTC_BKP_IDX, RTC_BKP_MAGIC);
    return 0;
}