
#include <stm32l4xx_hal.h>

static void init_fpu(void)
{
    SCB->CPACR |= (0xF << 20); // Enable CP10 and CP11 Full Access
    __DSB();
    __ISB();
}

static void enable_faults(void)
{
    SCB->SHCSR |= (SCB_SHCSR_MEMFAULTENA_Msk | SCB_SHCSR_USGFAULTENA_Msk | SCB_SHCSR_BUSFAULTENA_Msk);
    __ISB();
    __DSB();
}

static void init_systick(void)
{
    HAL_SYSTICK_Config(SystemCoreClock / 100); // Configure SysTick to interrupt every 1 ms
    HAL_SYSTICK_CLKSourceConfig(SYSTICK_CLKSOURCE_HCLK);
}

void init_hal(void)
{
    init_fpu();
    HAL_Init();

    enable_faults();

    init_systick();
}

void HAL_MspInit(void)
{
    HAL_NVIC_SetPriority(PendSV_IRQn, 15, 0);
    HAL_NVIC_SetPriority(SysTick_IRQn, 15, 0);

    __HAL_RCC_SYSCFG_CLK_ENABLE();
    __HAL_RCC_PWR_CLK_ENABLE();
}
