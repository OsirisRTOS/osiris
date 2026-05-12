#include "gpio.h"

#include "stm32l4xx_hal_gpio.h"
#include "stm32l4xx_hal_rcc.h"

void gpio_enable_clock(GPIO_TypeDef *port)
{
  if (port == GPIOA)
  {
    __HAL_RCC_GPIOA_CLK_ENABLE();
  }
  else if (port == GPIOB)
  {
    __HAL_RCC_GPIOB_CLK_ENABLE();
  }
  else if (port == GPIOC)
  {
    __HAL_RCC_GPIOC_CLK_ENABLE();
  }
  else if (port == GPIOD)
  {
    __HAL_RCC_GPIOD_CLK_ENABLE();
  }
  else if (port == GPIOE)
  {
    __HAL_RCC_GPIOE_CLK_ENABLE();
  }
  else if (port == GPIOF)
  {
    __HAL_RCC_GPIOF_CLK_ENABLE();
  }
  else if (port == GPIOG)
  {
    __HAL_RCC_GPIOG_CLK_ENABLE();
#if defined(GPIOH)
  }
  else if (port == GPIOH)
  {
    __HAL_RCC_GPIOH_CLK_ENABLE();
#endif
#if defined(GPIOI)
  }
  else if (port == GPIOI)
  {
    __HAL_RCC_GPIOI_CLK_ENABLE();
#endif
  }
}

void gpio_init_af(GPIO_TypeDef *port, uint16_t pin_mask, uint8_t af)
{
  GPIO_InitTypeDef gpio = {0};
  gpio.Mode = GPIO_MODE_AF_PP;
  gpio.Pull = GPIO_NOPULL;
  gpio.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
  gpio.Alternate = af;
  gpio.Pin = pin_mask;
  HAL_GPIO_Init(port, &gpio);
}

void gpio_init_af_od(GPIO_TypeDef *port, uint16_t pin_mask, uint8_t af)
{
  GPIO_InitTypeDef gpio = {0};
  gpio.Mode = GPIO_MODE_AF_OD;
  gpio.Pull = GPIO_NOPULL;
  gpio.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
  gpio.Alternate = af;
  gpio.Pin = pin_mask;
  HAL_GPIO_Init(port, &gpio);
}

void gpio_init_output(GPIO_TypeDef *port, uint16_t pin_mask)
{
  GPIO_InitTypeDef gpio = {0};
  gpio.Mode = GPIO_MODE_OUTPUT_PP;
  gpio.Pull = GPIO_NOPULL;
  gpio.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
  gpio.Pin = pin_mask;
  HAL_GPIO_Init(port, &gpio);
}

void gpio_init_output_od(GPIO_TypeDef *port, uint16_t pin_mask)
{
  GPIO_InitTypeDef gpio = {0};
  gpio.Mode = GPIO_MODE_OUTPUT_OD;
  gpio.Pull = GPIO_NOPULL;
  gpio.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
  gpio.Pin = pin_mask;
  HAL_GPIO_Init(port, &gpio);
}
