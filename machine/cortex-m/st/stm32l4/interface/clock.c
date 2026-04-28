#include "lib.h"
#include <stm32l4xx_hal.h>

static volatile uint64_t monotonic_hi = 0;

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

void init_clock_cfg(void)
{
    RCC_OscInitTypeDef RCC_OscInitStruct = {0};
    RCC_ClkInitTypeDef RCC_ClkInitStruct = {0};

    /* 80 MHz on STM32L4+ => Range 1 normal mode, not boost */
    __HAL_RCC_PWR_CLK_ENABLE();

    if (HAL_PWREx_ControlVoltageScaling(PWR_REGULATOR_VOLTAGE_SCALE1) != HAL_OK) {
        while (1) {}
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
        while (1) {}
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
        while (1) {}
    }

    SystemCoreClockUpdate();
    init_monotonic_timer();
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