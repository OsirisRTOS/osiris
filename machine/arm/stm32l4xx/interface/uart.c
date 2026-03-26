
#include "lib.h"
#include "stm32l4xx.h"
#include "stm32l4xx_hal_gpio.h"
#include "stm32l4xx_hal_rcc.h"
#include "stm32l4xx_hal_rcc_ex.h"

#include <stm32l4xx_hal.h>

static UART_HandleTypeDef HDBG_UART;

#ifndef OSIRIS_DEBUG_UART
  #error "OSIRIS_DEBUG_UART not defined."
#endif

int init_debug_uart(void) {
  HDBG_UART.Instance = OSIRIS_DEBUG_UART;
  HDBG_UART.Init.BaudRate = 115200;
  HDBG_UART.Init.Mode = UART_MODE_TX_RX;

  if (HAL_UART_Init(&HDBG_UART) != HAL_OK) {
    return -1;
  }

  return 0;
}

int write_debug_uart(const char *buf, int len) {
  if (HAL_UART_Transmit(&HDBG_UART, (uint8_t *)buf, len, 100) != HAL_OK) {
    return -1;
  }
  return len;
}

void HAL_UART_MspInit(UART_HandleTypeDef *huart) {
  RCC_PeriphCLKInitTypeDef PeriphClkInit = {0};
  GPIO_InitTypeDef GPIO_InitStruct = {0};
  GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
  GPIO_InitStruct.Pull = GPIO_NOPULL;
  GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_VERY_HIGH;

  if (huart->Instance == USART1) {
    // TX: PA9 (AF7), RX: PA10 (AF7) — Nucleo CN12 pins 21, 33
    PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_USART1;
    PeriphClkInit.Usart1ClockSelection = RCC_USART1CLKSOURCE_PCLK2;
    if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) return;
    __HAL_RCC_USART1_CLK_ENABLE();
    __HAL_RCC_GPIOA_CLK_ENABLE();
    GPIO_InitStruct.Alternate = GPIO_AF7_USART1;
    GPIO_InitStruct.Pin = GPIO_PIN_9 | GPIO_PIN_10;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);

  } else if (huart->Instance == USART2) {
    // TX: PA2 (AF7), RX: PA3 (AF7) — Nucleo CN12 pins 35, 37
    PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_USART2;
    PeriphClkInit.Usart2ClockSelection = RCC_USART2CLKSOURCE_PCLK1;
    if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) return;
    __HAL_RCC_USART2_CLK_ENABLE();
    __HAL_RCC_GPIOA_CLK_ENABLE();
    GPIO_InitStruct.Alternate = GPIO_AF7_USART2;
    GPIO_InitStruct.Pin = GPIO_PIN_2 | GPIO_PIN_3;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);

  } else if (huart->Instance == USART3) {
    // TX: PC10 (AF7), RX: PC11 (AF7) — Nucleo CN11 pins 1, 2
    PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_USART3;
    PeriphClkInit.Usart3ClockSelection = RCC_USART3CLKSOURCE_PCLK1;
    if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) return;
    __HAL_RCC_USART3_CLK_ENABLE();
    __HAL_RCC_GPIOC_CLK_ENABLE();
    GPIO_InitStruct.Alternate = GPIO_AF7_USART3;
    GPIO_InitStruct.Pin = GPIO_PIN_10 | GPIO_PIN_11;
    HAL_GPIO_Init(GPIOC, &GPIO_InitStruct);

  } else if (huart->Instance == UART4) {
    // TX: PA0 (AF8), RX: PA1 (AF8) — Nucleo CN11 pins 28, 30
    // Note: PA0 is also connected to user button B1 via SB197; cut SB197 to use UART4 TX
    PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_UART4;
    PeriphClkInit.Uart4ClockSelection = RCC_UART4CLKSOURCE_PCLK1;
    if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) return;
    __HAL_RCC_UART4_CLK_ENABLE();
    __HAL_RCC_GPIOA_CLK_ENABLE();
    GPIO_InitStruct.Alternate = GPIO_AF8_UART4;
    GPIO_InitStruct.Pin = GPIO_PIN_0 | GPIO_PIN_1;
    HAL_GPIO_Init(GPIOA, &GPIO_InitStruct);

  } else if (huart->Instance == UART5) {
    // TX: PC12 (AF8), RX: PD2 (AF8) — Nucleo CN11 pins 3, 4
    PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_UART5;
    PeriphClkInit.Uart5ClockSelection = RCC_UART5CLKSOURCE_PCLK1;
    if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) return;
    __HAL_RCC_UART5_CLK_ENABLE();
    __HAL_RCC_GPIOC_CLK_ENABLE();
    __HAL_RCC_GPIOD_CLK_ENABLE();
    GPIO_InitStruct.Alternate = GPIO_AF8_UART5;
    GPIO_InitStruct.Pin = GPIO_PIN_12;
    HAL_GPIO_Init(GPIOC, &GPIO_InitStruct);
    GPIO_InitStruct.Pin = GPIO_PIN_2;
    HAL_GPIO_Init(GPIOD, &GPIO_InitStruct);

  } else if (huart->Instance == LPUART1) {
    // TX: PG7 (AF8), RX: PG8 (AF8) — Nucleo CN12 pins 67, 66
    // Note: SB130/SB131 connect PG7/PG8 to the ST-LINK VCP by default;
    //       cut them to route LPUART1 to external pins instead.
    PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_LPUART1;
    PeriphClkInit.Lpuart1ClockSelection = RCC_LPUART1CLKSOURCE_PCLK1;
    if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) return;
    __HAL_RCC_LPUART1_CLK_ENABLE();
    HAL_PWREx_EnableVddIO2();
    __HAL_RCC_GPIOG_CLK_ENABLE();
    GPIO_InitStruct.Alternate = GPIO_AF8_LPUART1;
    GPIO_InitStruct.Pin = GPIO_PIN_7 | GPIO_PIN_8;
    HAL_GPIO_Init(GPIOG, &GPIO_InitStruct);
  }
}
