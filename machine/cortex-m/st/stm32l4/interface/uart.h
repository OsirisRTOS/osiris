#pragma once

#include <stdint.h>

#define UART_RX_RING_SZ 128
#define UART_TX_RING_SZ 256

typedef struct
{
    uintptr_t port;
    uint8_t pin;
    uint8_t af;
    uint16_t reserved;
} uart_pin_cfg_t;

typedef struct
{
    uintptr_t instance;
    /* tx and rx are required. rts/cts are optional; .port == 0 means unused. */
    uart_pin_cfg_t tx;
    uart_pin_cfg_t rx;
    uart_pin_cfg_t rts;
    uart_pin_cfg_t cts;
    uint32_t baud;
    uint8_t data_bits;    /* 7, 8, 9 */
    uint8_t stop_bits;    /* 1, 2 */
    uint8_t parity;       /* 0=none, 1=odd, 2=even */
    uint8_t flow_control; /* 0=none, 1=rts/cts */
    uint8_t irqn;         /* NVIC line; sourced from DT `interrupts` */
    uint8_t priority;     /* NVIC priority; sourced from DT `interrupts` */
} uart_bus_cfg_t;

typedef enum
{
    UART_IRQ_RX = 0,
    UART_IRQ_TX_DONE = 1,
} uart_irq_kind;

typedef void (*uart_irq_handler_fn)(uart_irq_kind kind, void *ctx);

/* Open a UART for IT-driven RX + IT-driven TX with software rings. Returns
 * 0..UART_SLOT_COUNT-1 on success or negative errno on failure (-EBUSY if
 * the instance is already initialised, -ENOMEM if no slot is free, -EINVAL
 * for malformed cfg). Idempotent: calling twice on the same instance with
 * an already-open slot returns the existing slot index. */
int uart_init(const uart_bus_cfg_t *cfg);

/* Open a UART in console (blocking-only, no IT, no rings) mode. The console
 * slot is reserved for `uprintln!` and panic output and refuses to be
 * promoted to IT mode by a later uart_init() on the same instance. */
int uart_init_console(const uart_bus_cfg_t *cfg);

/* Tear down a previously initialised slot. */
int uart_deinit(uintptr_t instance);

/* Blocking transmit. Bypasses the TX ring; calls HAL_UART_Transmit. Safe
 * for the console slot. timeout_ms == 0xFFFFFFFF means HAL_MAX_DELAY. */
int uart_transmit_blocking(uintptr_t instance,
                               const uint8_t *buf,
                               int len,
                               uint32_t timeout_ms);

/* Non-blocking transmit. Enqueues into the TX ring and arms HAL_UART_Transmit_IT.
 * Returns bytes enqueued (0..len) or -EAGAIN if the ring is full. Not valid
 * on a console-owned slot. */
int uart_transmit_nb(uintptr_t instance, const uint8_t *buf, int len);

/* Non-blocking receive. Drains up to len bytes from the RX ring. Returns
 * bytes drained (0..len). Not valid on a console-owned slot. */
int uart_receive_nb(uintptr_t instance, uint8_t *buf, int len);

/* Install an ISR-context callback. Called from HAL_UARTEx_RxEventCallback
 * once per RX event — a burst of bytes pushed to the RX ring on the IDLE
 * line or RXFIFO threshold (kind=RX) — and from HAL_UART_TxCpltCallback
 * when the TX ring drains (kind=TX_DONE). Pass NULL fn to clear. */
int uart_set_irq_handler(uintptr_t instance, uart_irq_handler_fn fn, void *ctx);

/* Returns the slot index for `instance` or -1 if not open. */
int uart_slot_of(uintptr_t instance);

/* Drives `HAL_UART_IRQHandler` for the slot at `slot_index`. Called from
 * the DT-generated `__irq_<N>_handler` trampolines in `uart_trampolines.h`.
 * Bounds-checks the slot index and ignores idle slots, so the trampolines
 * are safe to leave in place even when the slot was never opened. */
void uart_dispatch_by_slot(uint8_t slot_index);
