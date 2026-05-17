#pragma once

#include "stm32l4xx.h"

#include <stdint.h>

/* Edge bitmask passed to exti_configure. */
#define EXTI_EDGE_RISING_  0x1u
#define EXTI_EDGE_FALLING_ 0x2u

/* Configure an EXTI line for `line` (0..15) routed from `port` and unmask
 * it in NVIC. `edge_mask` is the bitwise OR of EXTI_EDGE_RISING_ and
 * EXTI_EDGE_FALLING_. NVIC priority is set to `priority`.
 *
 * Returns 0 on success or a negative PosixError code.
 *
 * `port` is a GPIO_TypeDef* but takes `void *` here to match the bindgen
 * surface in export.h. The peripheral-level interrupt is enabled, but the
 * Rust kernel owns the NVIC vector table — the caller is expected to
 * register a Rust handler at the corresponding vector slot. */
int exti_configure(void *port, uint8_t line, uint8_t edge_mask, uint8_t priority);

/* Mask `line` in IMR1 and clear any pending bit. NVIC is left enabled
 * (it may still be shared with other lines). */
int exti_release(uint8_t line);

/* Snapshot of EXTI_PR1 (16 GPIO lines plus a few peripheral lines we
 * don't use). The caller is expected to demux by line number. */
uint32_t exti_pending(void);

/* Clear pending bits in `mask` by writing them back to EXTI_PR1. */
void exti_ack(uint32_t mask);
