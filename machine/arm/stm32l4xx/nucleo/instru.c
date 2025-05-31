#define STM32L4R5xx
#include <stm32l4xx_hal.h>

void dwt_init(void)
{
    // Enable tracing.
    CoreDebug->DEMCR |= CoreDebug_DEMCR_TRCENA_Msk;
    DWT->CYCCNT = 0;

    // Enable the cycle counter.
    DWT->CTRL |= DWT_CTRL_CYCCNTENA_Msk;
}

void dwt_reset(void)
{
    DWT->CYCCNT = 0;
}

long dwt_read(void)
{
    return DWT->CYCCNT;
}

float dwt_cycles_to_ns(long cycles)
{
    long cpu_hz = HAL_RCC_GetHCLKFreq();
    return (float)cycles * ((float)1e9 / (float)cpu_hz);
}

float dwt_read_ns(void)
{
    return dwt_cycles_to_ns(DWT->CYCCNT);
}