#include "gpio.h"
#include "hal_api.h"

#include "stm32l4xx_hal_gpio.h"
#include "stm32l4xx_hal_rcc.h"

static int port_is_known(GPIO_TypeDef *port)
{
  if (port == NULL)
    return 0;
  if (port == GPIOA || port == GPIOB || port == GPIOC || port == GPIOD ||
      port == GPIOE || port == GPIOF || port == GPIOG)
    return 1;
#if defined(GPIOH)
  if (port == GPIOH)
    return 1;
#endif
#if defined(GPIOI)
  if (port == GPIOI)
    return 1;
#endif
  return 0;
}

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

static uint32_t pull_to_hal(uint8_t pull)
{
  switch (pull)
  {
  case GPIO_PULL_UP:
    return GPIO_PULLUP;
  case GPIO_PULL_DOWN:
    return GPIO_PULLDOWN;
  default:
    return GPIO_NOPULL;
  }
}

int gpio_configure_input(void *port, uint16_t pin_mask, uint8_t pull)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p) || pin_mask == 0)
    return -PosixError_EINVAL;
  if (pull > GPIO_PULL_DOWN)
    return -PosixError_EINVAL;

  gpio_enable_clock(p);
  GPIO_InitTypeDef gpio = {0};
  gpio.Mode = GPIO_MODE_INPUT;
  gpio.Pull = pull_to_hal(pull);
  gpio.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
  gpio.Pin = pin_mask;
  HAL_GPIO_Init(p, &gpio);
  return 0;
}

int gpio_configure_output_pp(void *port, uint16_t pin_mask, uint8_t initial)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p) || pin_mask == 0)
    return -PosixError_EINVAL;

  gpio_enable_clock(p);
  /* Drive the initial level before switching the pin to output mode so the
   * line never glitches the inverse polarity. */
  HAL_GPIO_WritePin(p, pin_mask, initial ? GPIO_PIN_SET : GPIO_PIN_RESET);
  GPIO_InitTypeDef gpio = {0};
  gpio.Mode = GPIO_MODE_OUTPUT_PP;
  gpio.Pull = GPIO_NOPULL;
  gpio.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
  gpio.Pin = pin_mask;
  HAL_GPIO_Init(p, &gpio);
  return 0;
}

int gpio_configure_output_od(void *port, uint16_t pin_mask, uint8_t initial,
                             uint8_t pull)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p) || pin_mask == 0)
    return -PosixError_EINVAL;
  if (pull > GPIO_PULL_DOWN)
    return -PosixError_EINVAL;

  gpio_enable_clock(p);
  /* Pre-drive ODR before switching mode so the line never glitches. */
  HAL_GPIO_WritePin(p, pin_mask, initial ? GPIO_PIN_SET : GPIO_PIN_RESET);
  GPIO_InitTypeDef gpio = {0};
  gpio.Mode = GPIO_MODE_OUTPUT_OD;
  gpio.Pull = pull_to_hal(pull);
  gpio.Speed = GPIO_SPEED_FREQ_VERY_HIGH;
  gpio.Pin = pin_mask;
  HAL_GPIO_Init(p, &gpio);
  return 0;
}

int gpio_write(void *port, uint16_t pin_mask, uint8_t level)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p) || pin_mask == 0)
    return -PosixError_EINVAL;
  HAL_GPIO_WritePin(p, pin_mask, level ? GPIO_PIN_SET : GPIO_PIN_RESET);
  return 0;
}

int gpio_read(void *port, uint16_t pin_mask)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p) || pin_mask == 0)
    return -PosixError_EINVAL;
  return HAL_GPIO_ReadPin(p, pin_mask) == GPIO_PIN_SET ? 1 : 0;
}

/* Read the output data register directly. Unlike gpio_read (which goes
 * through the input data register and returns 0 when the pin is in
 * analog mode — see RM0432 §8.3.12), this reflects the latched output
 * value regardless of the current pin mode. Use this when you need to
 * recover the previously commanded level on a pin that may not yet be
 * configured as output (e.g. `default-state = "keep"` at boot). The
 * port clock must already be ungated. */
int gpio_read_odr(void *port, uint16_t pin_mask)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p) || pin_mask == 0)
    return -PosixError_EINVAL;
  return (p->ODR & pin_mask) ? 1 : 0;
}

int gpio_toggle(void *port, uint16_t pin_mask)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p) || pin_mask == 0)
    return -PosixError_EINVAL;
  HAL_GPIO_TogglePin(p, pin_mask);
  return 0;
}

int gpio_clock_enable(void *port)
{
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!port_is_known(p))
    return -PosixError_EINVAL;
  gpio_enable_clock(p);
  return 0;
}
