#include "lib.h"
#include <assert.h>
#include <stm32l4xx_hal.h>
#include "stm32l4xx_hal_rcc.h"
#include <sys/_intsup.h>

static volatile uint64_t monotonic_hi = 0;

#define RTC_BKP_MAGIC 0x4F534952U
#define LSE_READY_TIMEOUT_LOOPS 2000000U
#define LSI_READY_TIMEOUT_LOOPS 200000U

#define ERROR_CONTROL_VOLTAGE_SCALING -1
#define ERROR_RCC_OSC_CONFIG -2
#define ERROR_RCC_CLOCK_CONFIG -3
#define ERROR_RTC_INIT_CLOCK_SOURCE -4
#define ERROR_RTC_INIT -5

static RTC_HandleTypeDef rtc_handle;

static int wait_rcc_ready_flag(uint32_t flag, uint32_t timeout_loops)
{
    while (timeout_loops > 0U) {
        if (__HAL_RCC_GET_FLAG(flag) != RESET) {
            return -1;
        }

        timeout_loops--;
    }

    return 0;
}

static HAL_StatusTypeDef select_rtc_clock_source(uint32_t source)
{
    RCC_PeriphCLKInitTypeDef periph = {0};

    periph.PeriphClockSelection = RCC_PERIPHCLK_RTC;
    periph.RTCClockSelection = source;

    return HAL_RCCEx_PeriphCLKConfig(&periph);
}

static int init_rtc_clock_source(void)
{
    __HAL_RCC_LSEDRIVE_CONFIG(RCC_LSEDRIVE_HIGH);
    __HAL_RCC_LSE_CONFIG(RCC_LSE_ON);

    if (!wait_rcc_ready_flag(RCC_FLAG_LSERDY, LSE_READY_TIMEOUT_LOOPS) &&
        select_rtc_clock_source(RCC_RTCCLKSOURCE_LSE) == HAL_OK) {
        __HAL_RCC_RTC_ENABLE();
        return 0;
    }
    
    __HAL_RCC_LSE_CONFIG(RCC_LSE_OFF);
    __HAL_RCC_LSI_ENABLE();

    if (!wait_rcc_ready_flag(RCC_FLAG_LSIRDY, LSI_READY_TIMEOUT_LOOPS) &&
        select_rtc_clock_source(RCC_RTCCLKSOURCE_LSI) == HAL_OK) {
        __HAL_RCC_RTC_ENABLE();
        return 0;
    }
    return -1;
}

int set_rtc_raw(unsigned long long raw);
int init_rtc(void)
{
    __HAL_RCC_PWR_CLK_ENABLE();
    HAL_PWR_EnableBkUpAccess();

    // TODO: setup Clock Security System
    if (init_rtc_clock_source()) {
        return ERROR_RTC_INIT_CLOCK_SOURCE;
    }

    rtc_handle.Instance = RTC;
    rtc_handle.Init.HourFormat = RTC_HOURFORMAT_24;
    rtc_handle.Init.AsynchPrediv = 0x7FU;
    rtc_handle.Init.SynchPrediv = 0x00FFU;
    rtc_handle.Init.OutPut = RTC_OUTPUT_DISABLE;
    rtc_handle.Init.OutPutRemap = RTC_OUTPUT_REMAP_NONE;
    rtc_handle.Init.OutPutPolarity = RTC_OUTPUT_POLARITY_HIGH;
    rtc_handle.Init.OutPutType = RTC_OUTPUT_TYPE_OPENDRAIN;

    if (HAL_RTC_Init(&rtc_handle) != HAL_OK) {
        return ERROR_RTC_INIT;
    }

    if (HAL_RTCEx_BKUPRead(&rtc_handle, RTC_BKP_DR0) != RTC_BKP_MAGIC) {
        // Sat 01.01.2000
        unsigned long long time = ((uint64_t)0) |
           ((uint64_t)0 << 8U) |
           ((uint64_t)0 << 16U) |
           ((uint64_t)RTC_WEEKDAY_SATURDAY << 32U) |              
           ((uint64_t)RTC_MONTH_JANUARY << 40U) |
           ((uint64_t)1 << 48U) |
           ((uint64_t)0 << 56U);
        return set_rtc_raw(time);
    }
    return 0;
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
    if ((TIM2->SR & TIM_SR_UIF) != 0U) {
        TIM2->SR &= ~TIM_SR_UIF;
        monotonic_hi += (1ULL << 32);
    }
}

int init_clock_cfg(void)
{
    RCC_OscInitTypeDef RCC_OscInitStruct = {0};
    RCC_ClkInitTypeDef RCC_ClkInitStruct = {0};

    /* 80 MHz on STM32L4+ => Range 1 normal mode, not boost */
    __HAL_RCC_PWR_CLK_ENABLE();

    if (HAL_PWREx_ControlVoltageScaling(PWR_REGULATOR_VOLTAGE_SCALE1) != HAL_OK) {
       return ERROR_CONTROL_VOLTAGE_SCALING;
    }

    /* HSI16 -> PLL -> 80 MHz SYSCLK */
    RCC_OscInitStruct.OscillatorType = RCC_OSCILLATORTYPE_HSI;
    RCC_OscInitStruct.HSIState = RCC_HSI_ON;
    RCC_OscInitStruct.HSICalibrationValue = RCC_HSICALIBRATION_DEFAULT;

    RCC_OscInitStruct.PLL.PLLState = RCC_PLL_ON;
    RCC_OscInitStruct.PLL.PLLSource = RCC_PLLSOURCE_HSI;
    RCC_OscInitStruct.PLL.PLLM = 1;
    RCC_OscInitStruct.PLL.PLLN = 10;
    RCC_OscInitStruct.PLL.PLLR = RCC_PLLR_DIV2;
    RCC_OscInitStruct.PLL.PLLP = RCC_PLLP_DIV7;   // arbitrary unless you use PLLP
    RCC_OscInitStruct.PLL.PLLQ = RCC_PLLQ_DIV2;   // arbitrary unless you use PLLQ

    if (HAL_RCC_OscConfig(&RCC_OscInitStruct) != HAL_OK) {
       return ERROR_RCC_OSC_CONFIG;
    }

    RCC_ClkInitStruct.ClockType =
        RCC_CLOCKTYPE_SYSCLK |
        RCC_CLOCKTYPE_HCLK   |
        RCC_CLOCKTYPE_PCLK1  |
        RCC_CLOCKTYPE_PCLK2;

    RCC_ClkInitStruct.SYSCLKSource   = RCC_SYSCLKSOURCE_PLLCLK;
    RCC_ClkInitStruct.AHBCLKDivider  = RCC_SYSCLK_DIV1;
    RCC_ClkInitStruct.APB1CLKDivider = RCC_HCLK_DIV1;
    RCC_ClkInitStruct.APB2CLKDivider = RCC_HCLK_DIV1;

    if (HAL_RCC_ClockConfig(&RCC_ClkInitStruct, FLASH_LATENCY_4) != HAL_OK) {
       return ERROR_RCC_CLOCK_CONFIG;
    }

    SystemCoreClockUpdate();
    init_monotonic_timer();
    return init_rtc();
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

long rtc_backup_register(unsigned index)
{
    return HAL_RTCEx_BKUPRead(&rtc_handle, RTC_BKP_DR0 + index);
}

void set_rtc_backup_register(unsigned index, long value)
{
    assert(index != 0 && "Register 0 is reserved for RTC init");
    HAL_RTCEx_BKUPWrite(&rtc_handle, RTC_BKP_DR0 + index, value);
}

unsigned long long rtc_raw(void)
{
    RTC_TimeTypeDef time = {0};
    RTC_DateTypeDef date = {0};

    if (HAL_RTC_GetTime(&rtc_handle, &time, RTC_FORMAT_BCD) != HAL_OK) {
        return -1U;
    }

    if (HAL_RTC_GetDate(&rtc_handle, &date, RTC_FORMAT_BCD) != HAL_OK) {
        return -2U;
    }

    return ((uint64_t)time.Hours) |
           ((uint64_t)time.Minutes << 8U) |
           ((uint64_t)time.Seconds << 16U) |
           ((uint64_t)date.WeekDay << 32U) |              
           ((uint64_t)date.Month << 40U) |
           ((uint64_t)date.Date << 48U) |
           ((uint64_t)date.Year << 56U);
}

int set_rtc_raw(unsigned long long raw)
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

    if (HAL_RTC_SetTime(&rtc_handle, &rtc_time, RTC_FORMAT_BCD) != HAL_OK) {
        return -1;
    }

    if (HAL_RTC_SetDate(&rtc_handle, &rtc_date, RTC_FORMAT_BCD) != HAL_OK) {
        return -2;
    }

    HAL_RTCEx_BKUPWrite(&rtc_handle, RTC_BKP_DR0, RTC_BKP_MAGIC);
    return 0;
}