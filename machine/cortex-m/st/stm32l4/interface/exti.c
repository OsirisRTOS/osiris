#include "exti.h"
#include "hal_api.h"
#include "stm32l4xx_hal_cortex.h"
#include "stm32l4xx_hal_exti.h"
#include "stm32l4xx_hal_rcc.h"

static int is_valid_port(GPIO_TypeDef *port)
{
  return port == GPIOA || port == GPIOB || port == GPIOC || port == GPIOD ||
         port == GPIOE || port == GPIOF || port == GPIOG
#if defined(GPIOH)
         || port == GPIOH
#endif
#if defined(GPIOI)
         || port == GPIOI
#endif
      ;
}

static uint32_t port_to_gpiosel(GPIO_TypeDef *port)
{
  if (port == GPIOA) return EXTI_GPIOA;
  if (port == GPIOB) return EXTI_GPIOB;
  if (port == GPIOC) return EXTI_GPIOC;
  if (port == GPIOD) return EXTI_GPIOD;
  if (port == GPIOE) return EXTI_GPIOE;
  if (port == GPIOF) return EXTI_GPIOF;
  if (port == GPIOG) return EXTI_GPIOG;
#if defined(GPIOH)
  if (port == GPIOH) return EXTI_GPIOH;
#endif
#if defined(GPIOI)
  if (port == GPIOI) return EXTI_GPIOI;
#endif
  return EXTI_GPIOA;
}

/* Precondition: line < 16. */
static IRQn_Type irqn_for_line(uint8_t line)
{
  switch (line)
  {
  case 0: return EXTI0_IRQn;
  case 1: return EXTI1_IRQn;
  case 2: return EXTI2_IRQn;
  case 3: return EXTI3_IRQn;
  case 4: return EXTI4_IRQn;
  case 5:
  case 6:
  case 7:
  case 8:
  case 9: return EXTI9_5_IRQn;
  case 10:
  case 11:
  case 12:
  case 13:
  case 14:
  case 15: return EXTI15_10_IRQn;
  }
  return (IRQn_Type)0;
}

/* Map our edge bitmask to the HAL's EXTI_TRIGGER_* values. The bit
 * positions happen to match, but the explicit lookup keeps us robust
 * against the HAL renumbering them. */
static uint32_t edge_mask_to_trigger(uint8_t edge_mask)
{
  uint32_t trigger = 0;
  if (edge_mask & EXTI_EDGE_RISING_)  trigger |= EXTI_TRIGGER_RISING;
  if (edge_mask & EXTI_EDGE_FALLING_) trigger |= EXTI_TRIGGER_FALLING;
  return trigger;
}

int exti_configure(void *port, uint8_t line, uint8_t edge_mask, uint8_t priority)
{
  if (line >= 16u)
  {
    return -PosixError_EINVAL;
  }
  uint32_t trigger = edge_mask_to_trigger(edge_mask);
  if (trigger == 0u)
  {
    /* No edge selected would unmask a line that can never fire. */
    return -PosixError_EINVAL;
  }
  GPIO_TypeDef *p = (GPIO_TypeDef *)port;
  if (!is_valid_port(p))
  {
    return -PosixError_EINVAL;
  }

  __HAL_RCC_SYSCFG_CLK_ENABLE();

  /* HAL_EXTI_SetConfigLine writes SYSCFG_EXTICR, RTSR1/FTSR1, and IMR1
   * for us. The handle's PendingCallback is unused — we run our own
   * dispatcher and never call HAL_EXTI_IRQHandler. */
  EXTI_HandleTypeDef hexti = {0};
  EXTI_ConfigTypeDef cfg = {
      .Line    = EXTI_LINE_0 | (uint32_t)line,
      .Mode    = EXTI_MODE_INTERRUPT,
      .Trigger = trigger,
      .GPIOSel = port_to_gpiosel(p),
  };
  if (HAL_EXTI_SetConfigLine(&hexti, &cfg) != HAL_OK)
  {
    return -PosixError_EIO;
  }

  /* Clear any latent pending edge from before this configuration so
   * the first IRQ corresponds to a real edge. */
  HAL_EXTI_ClearPending(&hexti, EXTI_TRIGGER_RISING_FALLING);

  IRQn_Type irqn = irqn_for_line(line);
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
  EXTI_HandleTypeDef hexti = { .Line = EXTI_LINE_0 | (uint32_t)line };
  if (HAL_EXTI_ClearConfigLine(&hexti) != HAL_OK)
  {
    return -PosixError_EIO;
  }
  HAL_EXTI_ClearPending(&hexti, EXTI_TRIGGER_RISING_FALLING);
  /* NVIC vector is left enabled; it may still serve other lines that
   * share it (5..9, 10..15). */
  return 0;
}

/* Raw access: the dispatcher needs every bit of EXTI_PR1 in one read
 * so it can demux several lines latched in the same IRQ. */
uint32_t exti_pending(void)
{
  return EXTI->PR1;
}

void exti_ack(uint32_t mask)
{
  EXTI->PR1 = mask;
}
