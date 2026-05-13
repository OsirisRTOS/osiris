#include "exti.h"
#include "hal_api.h"
#include "stm32l4xx_hal_cortex.h"
#include "stm32l4xx_hal_rcc.h"

static uint8_t port_index(GPIO_TypeDef *port)
{
  if (port == GPIOA)
    return 0;
  if (port == GPIOB)
    return 1;
  if (port == GPIOC)
    return 2;
  if (port == GPIOD)
    return 3;
  if (port == GPIOE)
    return 4;
  if (port == GPIOF)
    return 5;
  if (port == GPIOG)
    return 6;
#if defined(GPIOH)
  if (port == GPIOH)
    return 7;
#endif
#if defined(GPIOI)
  if (port == GPIOI)
    return 8;
#endif
  return 0xFFu;
}

static IRQn_Type exti_irqn_for_line(uint8_t line)
{
  switch (line)
  {
  case 0:
    return EXTI0_IRQn;
  case 1:
    return EXTI1_IRQn;
  case 2:
    return EXTI2_IRQn;
  case 3:
    return EXTI3_IRQn;
  case 4:
    return EXTI4_IRQn;
  case 5:
  case 6:
  case 7:
  case 8:
  case 9:
    return EXTI9_5_IRQn;
  case 10:
  case 11:
  case 12:
  case 13:
  case 14:
  case 15:
    return EXTI15_10_IRQn;
  default:
    /* Unreachable; guarded above. */
    return (IRQn_Type)0;
  }
}

int exti_configure(void *port, uint8_t line, uint8_t edge_mask, uint8_t priority)
{
  if (line >= 16u)
  {
    return -PosixError_EINVAL;
  }
  if ((edge_mask & (EXTI_EDGE_RISING_ | EXTI_EDGE_FALLING_)) == 0u)
  {
    /* No edge selected would unmask a line that can never fire. */
    return -PosixError_EINVAL;
  }
  uint8_t port_idx = port_index((GPIO_TypeDef *)port);
  if (port_idx == 0xFFu)
  {
    return -PosixError_EINVAL;
  }

  __HAL_RCC_SYSCFG_CLK_ENABLE();

  uint32_t line_mask = 1u << line;
  uint32_t exticr_idx = line >> 2;            /* line / 4 */
  uint32_t exticr_pos = (uint32_t)(line & 3) * 4u;

  uint32_t primask = __get_PRIMASK();
  __disable_irq();

  uint32_t exticr = SYSCFG->EXTICR[exticr_idx];
  exticr &= ~(0xFu << exticr_pos);
  exticr |= ((uint32_t)port_idx) << exticr_pos;
  SYSCFG->EXTICR[exticr_idx] = exticr;

  if (edge_mask & EXTI_EDGE_RISING_)
  {
    EXTI->RTSR1 |= line_mask;
  }
  else
  {
    EXTI->RTSR1 &= ~line_mask;
  }
  if (edge_mask & EXTI_EDGE_FALLING_)
  {
    EXTI->FTSR1 |= line_mask;
  }
  else
  {
    EXTI->FTSR1 &= ~line_mask;
  }

  /* Clear any spurious pending bit before unmasking. */
  EXTI->PR1 = line_mask;
  EXTI->IMR1 |= line_mask;

  __set_PRIMASK(primask);

  IRQn_Type irqn = exti_irqn_for_line(line);
  HAL_NVIC_SetPriority(irqn, priority, 0);
  HAL_NVIC_EnableIRQ(irqn);

  return 0;
}

int exti_release(uint8_t line)
{
  if (line >= 16u)
  {
    return -PosixError_EINVAL;
  }
  uint32_t line_mask = 1u << line;

  uint32_t primask = __get_PRIMASK();
  __disable_irq();
  EXTI->IMR1 &= ~line_mask;
  EXTI->RTSR1 &= ~line_mask;
  EXTI->FTSR1 &= ~line_mask;
  EXTI->PR1 = line_mask;
  __set_PRIMASK(primask);

  return 0;
}

uint32_t exti_pending(void)
{
  return EXTI->PR1;
}

void exti_ack(uint32_t mask)
{
  EXTI->PR1 = mask;
}
