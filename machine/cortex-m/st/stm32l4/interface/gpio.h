#pragma once

#include "stm32l4xx.h"

#include <stdint.h>

void gpio_enable_clock(GPIO_TypeDef *port);
void gpio_init_af(GPIO_TypeDef *port, uint16_t pin_mask, uint8_t af);
void gpio_init_af_od(GPIO_TypeDef *port, uint16_t pin_mask, uint8_t af);
void gpio_init_output(GPIO_TypeDef *port, uint16_t pin_mask);
