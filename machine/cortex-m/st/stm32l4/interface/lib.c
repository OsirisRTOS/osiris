#include "lib.h"
#include <stm32l4xx_hal.h>

static void init_fpu(void) {
  SCB->CPACR |= (0xF << 20); // Enable CP10 and CP11 Full Access
  __DSB();
  __ISB();
}

static void enable_faults(void) {
  SCB->SHCSR |= (SCB_SHCSR_MEMFAULTENA_Msk | SCB_SHCSR_USGFAULTENA_Msk |
                 SCB_SHCSR_BUSFAULTENA_Msk);
  __ISB();
  __DSB();
}

static int init_systick(void) {
  if (HAL_SYSTICK_Config(SystemCoreClock / 1000)) // Configure SysTick to interrupt every 1 ms
    return -1;
  HAL_SYSTICK_CLKSourceConfig(SYSTICK_CLKSOURCE_HCLK);
  return 0;
}

unsigned long long systick_freq(void) {
  return 1000;
}

int init_hal(void) {
#if OSIRIS_TUNING_ENABLEFPU
  init_fpu();
#endif
  HAL_Init();

  enable_faults();

  int ret = init_clock_cfg();
  if (ret != 0) {
    return ret;
  }

  ret = init_systick();
  if (ret != 0) {
    return ret;
  }

  return 0;
}

void HAL_MspInit(void) {
  HAL_NVIC_SetPriority(PendSV_IRQn, 15, 0);
  HAL_NVIC_SetPriority(SysTick_IRQn, 15, 0);

  __HAL_RCC_SYSCFG_CLK_ENABLE();
  __HAL_RCC_PWR_CLK_ENABLE();
}
