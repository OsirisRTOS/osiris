#pragma once

#include "export.h" /* GPIO_PULL_NONE / GPIO_PULL_UP / GPIO_PULL_DOWN */
#include "stm32l4xx.h"

#include <stdint.h>

void gpio_enable_clock(GPIO_TypeDef *port);
void gpio_init_af(GPIO_TypeDef *port, uint16_t pin_mask, uint8_t af);
void gpio_init_af_od(GPIO_TypeDef *port, uint16_t pin_mask, uint8_t af);
void gpio_init_output(GPIO_TypeDef *port, uint16_t pin_mask);
void gpio_init_output_od(GPIO_TypeDef *port, uint16_t pin_mask);

/* The functions below take `void *port` rather than `GPIO_TypeDef *` so
 * the same declarations appear in `export.h` (consumed by bindgen) — that
 * lets bindgen avoid pulling in the full STM32 HAL header tree. The .c
 * file casts to GPIO_TypeDef* internally.
 *
 * All return 0 on success or a negative PosixError code on failure. */

int gpio_configure_input(void *port, uint16_t pin_mask, uint8_t pull);
int gpio_configure_output_pp(void *port, uint16_t pin_mask, uint8_t initial);
int gpio_write(void *port, uint16_t pin_mask, uint8_t level);
/* gpio_read returns 0 or 1 (level) on success, or negative PosixError. */
int gpio_read(void *port, uint16_t pin_mask);
int gpio_toggle(void *port, uint16_t pin_mask);
int gpio_clock_enable(void *port);
