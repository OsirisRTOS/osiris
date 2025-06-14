
#include <stm32l4xx_hal.h>

static UART_HandleTypeDef HLPUART1;

int init_debug_uart(void)
{
    HLPUART1.Instance = LPUART1;
    HLPUART1.Init.BaudRate = 115200;
    HLPUART1.Init.Mode = UART_MODE_TX_RX;

    if (HAL_UART_Init(&HLPUART1) != HAL_OK) {
        return -1;
    }

    return 0;
}

int write_debug_uart(const char *buf, int len)
{
    if (HAL_UART_Transmit(&HLPUART1, (uint8_t *)buf, len, 100) != HAL_OK) {
        return -1;
    }
    return len; // Return number of bytes written
}

void HAL_UART_MspInit(UART_HandleTypeDef *huart)
{
    if (huart->Instance == LPUART1) {

        RCC_PeriphCLKInitTypeDef PeriphClkInit = {0};
        PeriphClkInit.PeriphClockSelection = RCC_PERIPHCLK_LPUART1;
        PeriphClkInit.Lpuart1ClockSelection = RCC_LPUART1CLKSOURCE_PCLK1;

        if (HAL_RCCEx_PeriphCLKConfig(&PeriphClkInit) != HAL_OK) {
            return;
        }

        __HAL_RCC_LPUART1_CLK_ENABLE();

        GPIO_InitTypeDef GPIO_InitStruct = {0};
        GPIO_InitStruct.Pin = GPIO_PIN_6 | GPIO_PIN_7; // LPUART1_TX, LPUART1_RX
        GPIO_InitStruct.Mode = GPIO_MODE_AF_PP;
        GPIO_InitStruct.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
        GPIO_InitStruct.Alternate = GPIO_AF8_LPUART1;

        HAL_PWREx_EnableVddIO2();
        __HAL_RCC_GPIOG_CLK_ENABLE();
        HAL_GPIO_Init(GPIOG, &GPIO_InitStruct);
    }
}