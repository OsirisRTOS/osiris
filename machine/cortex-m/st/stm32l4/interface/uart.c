#include "uart.h"

#include "lib.h"
#include "gpio.h"
#include "stm32l4xx.h"
#include "stm32l4xx_hal.h"
#include "stm32l4xx_hal_gpio.h"
#include "stm32l4xx_hal_pwr_ex.h"
#include "stm32l4xx_hal_rcc.h"
#include "stm32l4xx_hal_rcc_ex.h"
#include "stm32l4xx_hal_uart.h"

#define UART_SLOT_COUNT 6

#define UART_ERR_OK 0
#define UART_ERR_INVAL (-1)
#define UART_ERR_NOMEM (-2)
#define UART_ERR_BUSY (-3)
#define UART_ERR_AGAIN (-4)
#define UART_ERR_IO (-5)

/* RX coalescing: HAL_UARTEx_ReceiveToIdle_IT lands bytes here and flushes
 * on the hardware IDLE line (~1 char time after the last byte) or when
 * full. 32 bounds worst-case tail latency (~2.8 ms @ 115200) for a
 * gapless >32-byte stream; record-oriented traffic flushes at the
 * inter-record gap. Must be <= UART_RX_RING_SZ - 1 (in-ISR copy target). */
#define UART_RX_SCRATCH_SZ 32
/* Mid-stream RXFIFO threshold IRQ guards against overrun on long bursts;
 * IDLE handles prompt end-of-frame delivery. TXFIFO threshold batches TX
 * IT (no TX state-machine change needed). */
#define UART_RX_FIFO_THRESH UART_RXFIFO_THRESHOLD_3_4
#define UART_TX_FIFO_THRESH UART_TXFIFO_THRESHOLD_1_2

typedef struct
{
    uint8_t in_use;
    uint8_t console_owned;
    uart_bus_cfg_t bus_cfg;
    UART_HandleTypeDef huart;

    /* ReceiveToIdle landing buffer; drained into rx_ring in
     * HAL_UARTEx_RxEventCallback on IDLE/threshold, then re-armed. */
    uint8_t rx_scratch[UART_RX_SCRATCH_SZ];
    uint8_t rx_ring[UART_RX_RING_SZ];
    volatile uint16_t rx_head;
    volatile uint16_t rx_tail;

    uint8_t tx_ring[UART_TX_RING_SZ];
    volatile uint16_t tx_head;
    volatile uint16_t tx_tail;
    volatile uint16_t tx_in_flight;
    volatile uint8_t tx_busy;

    uart_irq_handler_fn cb;
    void *cb_ctx;
} uart_slot_t;

static uart_slot_t uart_slots[UART_SLOT_COUNT];

static uart_slot_t *uart_find_slot(uintptr_t instance)
{
    for (int i = 0; i < UART_SLOT_COUNT; ++i)
    {
        if (uart_slots[i].in_use && uart_slots[i].bus_cfg.instance == instance)
        {
            return &uart_slots[i];
        }
    }
    return NULL;
}

static int uart_slot_index(const uart_slot_t *s)
{
    return (int)(s - uart_slots);
}

static uart_slot_t *uart_alloc_slot(void)
{
    for (int i = 0; i < UART_SLOT_COUNT; ++i)
    {
        if (!uart_slots[i].in_use)
        {
            return &uart_slots[i];
        }
    }
    return NULL;
}

static int uart_periph_clock_enable(USART_TypeDef *u)
{
    RCC_PeriphCLKInitTypeDef init = {0};
#if defined(USART1)
    if (u == USART1)
    {
        init.PeriphClockSelection = RCC_PERIPHCLK_USART1;
        init.Usart1ClockSelection = RCC_USART1CLKSOURCE_PCLK2;
        if (HAL_RCCEx_PeriphCLKConfig(&init) != HAL_OK)
            return UART_ERR_IO;
        __HAL_RCC_USART1_CLK_ENABLE();
        return UART_ERR_OK;
    }
#endif
#if defined(USART2)
    if (u == USART2)
    {
        init.PeriphClockSelection = RCC_PERIPHCLK_USART2;
        init.Usart2ClockSelection = RCC_USART2CLKSOURCE_PCLK1;
        if (HAL_RCCEx_PeriphCLKConfig(&init) != HAL_OK)
            return UART_ERR_IO;
        __HAL_RCC_USART2_CLK_ENABLE();
        return UART_ERR_OK;
    }
#endif
#if defined(USART3)
    if (u == USART3)
    {
        init.PeriphClockSelection = RCC_PERIPHCLK_USART3;
        init.Usart3ClockSelection = RCC_USART3CLKSOURCE_PCLK1;
        if (HAL_RCCEx_PeriphCLKConfig(&init) != HAL_OK)
            return UART_ERR_IO;
        __HAL_RCC_USART3_CLK_ENABLE();
        return UART_ERR_OK;
    }
#endif
#if defined(UART4)
    if (u == UART4)
    {
        init.PeriphClockSelection = RCC_PERIPHCLK_UART4;
        init.Uart4ClockSelection = RCC_UART4CLKSOURCE_PCLK1;
        if (HAL_RCCEx_PeriphCLKConfig(&init) != HAL_OK)
            return UART_ERR_IO;
        __HAL_RCC_UART4_CLK_ENABLE();
        return UART_ERR_OK;
    }
#endif
#if defined(UART5)
    if (u == UART5)
    {
        init.PeriphClockSelection = RCC_PERIPHCLK_UART5;
        init.Uart5ClockSelection = RCC_UART5CLKSOURCE_PCLK1;
        if (HAL_RCCEx_PeriphCLKConfig(&init) != HAL_OK)
            return UART_ERR_IO;
        __HAL_RCC_UART5_CLK_ENABLE();
        return UART_ERR_OK;
    }
#endif
#if defined(LPUART1)
    if (u == LPUART1)
    {
        init.PeriphClockSelection = RCC_PERIPHCLK_LPUART1;
        init.Lpuart1ClockSelection = RCC_LPUART1CLKSOURCE_PCLK1;
        if (HAL_RCCEx_PeriphCLKConfig(&init) != HAL_OK)
            return UART_ERR_IO;
        __HAL_RCC_LPUART1_CLK_ENABLE();
        /* L4Rxxx LPUART1 pins (PG7/PG8) are on VddIO2; without this they
         * float and TX never leaves the pad. */
        HAL_PWREx_EnableVddIO2();
        return UART_ERR_OK;
    }
#endif
    return UART_ERR_INVAL;
}

static uint32_t uart_word_length(uint8_t data_bits, uint8_t parity)
{
    /* STM32 word length includes the parity bit. */
    uint8_t total = data_bits + (parity == 0 ? 0 : 1);
    switch (total)
    {
    case 7:
        return UART_WORDLENGTH_7B;
    case 8:
        return UART_WORDLENGTH_8B;
    case 9:
        return UART_WORDLENGTH_9B;
    default:
        return UART_WORDLENGTH_8B;
    }
}

static uint32_t uart_stop_bits(uint8_t bits)
{
    return bits == 2 ? UART_STOPBITS_2 : UART_STOPBITS_1;
}

static uint32_t uart_parity_mode(uint8_t p)
{
    switch (p)
    {
    case 1:
        return UART_PARITY_ODD;
    case 2:
        return UART_PARITY_EVEN;
    default:
        return UART_PARITY_NONE;
    }
}

static uint32_t uart_hw_flow(uint8_t f)
{
    return f == 1 ? UART_HWCONTROL_RTS_CTS : UART_HWCONTROL_NONE;
}

static int uart_apply_pin(const uart_pin_cfg_t *pin)
{
    if (pin->port == 0)
        return UART_ERR_OK;
    GPIO_TypeDef *port = (GPIO_TypeDef *)pin->port;
    gpio_enable_clock(port);
    gpio_init_af(port, (uint16_t)(1u << pin->pin), pin->af);
    return UART_ERR_OK;
}

static int uart_hw_init(uart_slot_t *slot)
{
    USART_TypeDef *u = (USART_TypeDef *)slot->bus_cfg.instance;

    if (uart_periph_clock_enable(u) != UART_ERR_OK)
        return UART_ERR_INVAL;

    uart_apply_pin(&slot->bus_cfg.tx);
    uart_apply_pin(&slot->bus_cfg.rx);
    if (slot->bus_cfg.flow_control == 1)
    {
        uart_apply_pin(&slot->bus_cfg.rts);
        uart_apply_pin(&slot->bus_cfg.cts);
    }

    slot->huart.Instance = u;
    slot->huart.Init.BaudRate = slot->bus_cfg.baud ? slot->bus_cfg.baud : 115200u;
    slot->huart.Init.WordLength = uart_word_length(
        slot->bus_cfg.data_bits ? slot->bus_cfg.data_bits : 8,
        slot->bus_cfg.parity);
    slot->huart.Init.StopBits = uart_stop_bits(slot->bus_cfg.stop_bits);
    slot->huart.Init.Parity = uart_parity_mode(slot->bus_cfg.parity);
    slot->huart.Init.Mode = UART_MODE_TX_RX;
    slot->huart.Init.HwFlowCtl = uart_hw_flow(slot->bus_cfg.flow_control);
    slot->huart.Init.OverSampling = UART_OVERSAMPLING_16;
    slot->huart.Init.OneBitSampling = UART_ONE_BIT_SAMPLE_DISABLE;
    slot->huart.AdvancedInit.AdvFeatureInit = UART_ADVFEATURE_NO_INIT;

    if (HAL_UART_Init(&slot->huart) != HAL_OK)
        return UART_ERR_IO;

    /* FIFO mode for the interrupt-driven (non-console) path only. The
     * console is blocking and stays register-pristine. Thresholds must
     * be set while FIFO is still disabled (they program CR3 RX/TXFTCFG);
     * EnableFifoMode then sets CR1 FIFOEN and recomputes the HAL's
     * Nb{Rx,Tx}DataToProcess from those thresholds. All three require
     * post-HAL_UART_Init state. */
    if (!slot->console_owned)
    {
        if (HAL_UARTEx_SetTxFifoThreshold(&slot->huart, UART_TX_FIFO_THRESH) != HAL_OK)
            return UART_ERR_IO;
        if (HAL_UARTEx_SetRxFifoThreshold(&slot->huart, UART_RX_FIFO_THRESH) != HAL_OK)
            return UART_ERR_IO;
        if (HAL_UARTEx_EnableFifoMode(&slot->huart) != HAL_OK)
            return UART_ERR_IO;
    }

    return UART_ERR_OK;
}

/* Override the HAL's __weak MspInit: pins/clock are configured in
 * uart_hw_init before HAL_UART_Init, so MspInit has nothing to do. */
void HAL_UART_MspInit(UART_HandleTypeDef *huart)
{
    (void)huart;
}

int uart_slot_of(uintptr_t instance)
{
    uart_slot_t *s = uart_find_slot(instance);
    return s ? uart_slot_index(s) : -1;
}

static int uart_init_common(const uart_bus_cfg_t *cfg, uint8_t console_owned)
{
    if (cfg == NULL || cfg->instance == 0)
        return UART_ERR_INVAL;

    uart_slot_t *existing = uart_find_slot(cfg->instance);
    if (existing != NULL)
    {
        /* Idempotent re-open, but the console slot can't be promoted
         * to IT mode (and vice versa) — they have different invariants. */
        if (existing->console_owned != console_owned)
            return UART_ERR_BUSY;
        return uart_slot_index(existing);
    }

    uart_slot_t *slot = uart_alloc_slot();
    if (slot == NULL)
        return UART_ERR_NOMEM;

    *slot = (uart_slot_t){0};
    slot->bus_cfg = *cfg;
    slot->console_owned = console_owned;

    if (uart_hw_init(slot) != UART_ERR_OK)
        return UART_ERR_IO;

    if (!console_owned)
    {
        IRQn_Type irqn = (IRQn_Type)cfg->irqn;
        HAL_NVIC_SetPriority(irqn, cfg->priority, 0);
        HAL_NVIC_EnableIRQ(irqn);
        /* Arm idle-line RX: the HAL fires HAL_UARTEx_RxEventCallback with
         * the bytes received so far on the IDLE line or when rx_scratch
         * fills, instead of one RxCplt per byte. */
        if (HAL_UARTEx_ReceiveToIdle_IT(&slot->huart, slot->rx_scratch,
                                        UART_RX_SCRATCH_SZ) != HAL_OK)
            return UART_ERR_IO;
    }

    /* in_use is the last write: a failure above leaves the slot free, so
     * uart_find_slot/uart_alloc_slot never hand out a half-initialized
     * slot (fixes the failed-init-leaves-slot-stuck-in-use bug). */
    slot->in_use = 1;

    return uart_slot_index(slot);
}

int uart_init(const uart_bus_cfg_t *cfg)
{
    return uart_init_common(cfg, 0);
}

int uart_init_console(const uart_bus_cfg_t *cfg)
{
    return uart_init_common(cfg, 1);
}

int uart_deinit(uintptr_t instance)
{
    uart_slot_t *slot = uart_find_slot(instance);
    if (slot == NULL)
        return UART_ERR_OK;

    if (!slot->console_owned)
        HAL_NVIC_DisableIRQ((IRQn_Type)slot->bus_cfg.irqn);
    HAL_UART_DeInit(&slot->huart);
    slot->in_use = 0;
    slot->console_owned = 0;
    return UART_ERR_OK;
}

int uart_transmit_blocking(uintptr_t instance,
                               const uint8_t *buf,
                               int len,
                               uint32_t timeout_ms)
{
    if (buf == NULL || len < 0)
        return UART_ERR_INVAL;
    uart_slot_t *slot = uart_find_slot(instance);
    if (slot == NULL)
        return UART_ERR_INVAL;
    if (len == 0)
        return 0;

    HAL_StatusTypeDef rc = HAL_UART_Transmit(&slot->huart, (uint8_t *)buf, (uint16_t)len, timeout_ms);
    if (rc != HAL_OK)
        return UART_ERR_IO;
    return len;
}

/* head == tail is empty; (head + 1) % cap == tail is full. One slot
 * reserved as the empty/full sentinel. */
static uint16_t ring_occ(uint16_t head, uint16_t tail, uint16_t cap)
{
    return (uint16_t)((head + cap - tail) % cap);
}

static uint16_t ring_free(uint16_t head, uint16_t tail, uint16_t cap)
{
    return (uint16_t)(cap - 1u - ring_occ(head, tail, cap));
}

static void uart_arm_tx(uart_slot_t *slot);

int uart_transmit_nb(uintptr_t instance, const uint8_t *buf, int len)
{
    if (buf == NULL || len < 0)
        return UART_ERR_INVAL;
    uart_slot_t *slot = uart_find_slot(instance);
    if (slot == NULL || slot->console_owned)
        return UART_ERR_INVAL;
    if (len == 0)
        return 0;

    /* IRQ-disable: head/tail are shared with TxCpltCallback. */
    uint32_t pri = __get_PRIMASK();
    __disable_irq();

    uint16_t free = ring_free(slot->tx_head, slot->tx_tail, UART_TX_RING_SZ);
    if (free == 0)
    {
        if (!pri)
            __enable_irq();
        return UART_ERR_AGAIN;
    }

    int n = (int)free < len ? (int)free : len;
    for (int i = 0; i < n; ++i)
    {
        slot->tx_ring[slot->tx_head] = buf[i];
        slot->tx_head = (uint16_t)((slot->tx_head + 1) % UART_TX_RING_SZ);
    }

    if (!slot->tx_busy)
        uart_arm_tx(slot);

    if (!pri)
        __enable_irq();
    return n;
}

int uart_receive_nb(uintptr_t instance, uint8_t *buf, int len)
{
    if (buf == NULL || len < 0)
        return UART_ERR_INVAL;
    uart_slot_t *slot = uart_find_slot(instance);
    if (slot == NULL || slot->console_owned)
        return UART_ERR_INVAL;
    if (len == 0)
        return 0;

    uint32_t pri = __get_PRIMASK();
    __disable_irq();

    int n = 0;
    while (n < len && slot->rx_tail != slot->rx_head)
    {
        buf[n++] = slot->rx_ring[slot->rx_tail];
        slot->rx_tail = (uint16_t)((slot->rx_tail + 1) % UART_RX_RING_SZ);
    }

    if (!pri)
        __enable_irq();
    return n;
}

int uart_set_irq_handler(uintptr_t instance, uart_irq_handler_fn fn, void *ctx)
{
    uart_slot_t *slot = uart_find_slot(instance);
    if (slot == NULL)
        return UART_ERR_INVAL;
    if (slot->console_owned)
        return UART_ERR_BUSY;
    uint32_t pri = __get_PRIMASK();
    __disable_irq();
    slot->cb = fn;
    slot->cb_ctx = ctx;
    if (!pri)
        __enable_irq();
    return UART_ERR_OK;
}

/* Arms the next TX-IT chunk if the ring has data. Caller must hold IRQs off.
 * Sends the largest contiguous span up to end-of-ring; TxCpltCallback re-arms
 * for any wrap-around. */
static void uart_arm_tx(uart_slot_t *slot)
{
    if (slot->tx_head == slot->tx_tail)
    {
        slot->tx_busy = 0;
        slot->tx_in_flight = 0;
        return;
    }
    uint16_t span;
    if (slot->tx_tail < slot->tx_head)
        span = (uint16_t)(slot->tx_head - slot->tx_tail);
    else
        span = (uint16_t)(UART_TX_RING_SZ - slot->tx_tail);

    slot->tx_busy = 1;
    slot->tx_in_flight = span;
    if (HAL_UART_Transmit_IT(&slot->huart, &slot->tx_ring[slot->tx_tail], span) != HAL_OK)
    {
        slot->tx_busy = 0;
        slot->tx_in_flight = 0;
    }
}

/* Fires once per RX event (IDLE line, RXFIFO threshold, or rx_scratch
 * full) with the number of bytes landed in rx_scratch — not per byte.
 * IDLE and buffer-full are handled identically (no half-transfer split,
 * so HAL_UARTEx_GetRxEventType is unnecessary). */
void HAL_UARTEx_RxEventCallback(UART_HandleTypeDef *huart, uint16_t Size)
{
    for (int i = 0; i < UART_SLOT_COUNT; ++i)
    {
        uart_slot_t *slot = &uart_slots[i];
        if (!slot->in_use || &slot->huart != huart)
            continue;

        /* HAL bounds Size to the armed length; clamp defensively. */
        if (Size > UART_RX_SCRATCH_SZ)
            Size = UART_RX_SCRATCH_SZ;

        for (uint16_t j = 0; j < Size; ++j)
        {
            uint16_t next = (uint16_t)((slot->rx_head + 1) % UART_RX_RING_SZ);
            if (next == slot->rx_tail)
                break; /* ring full ⇒ drop rest; consumer resyncs. */
            slot->rx_ring[slot->rx_head] = slot->rx_scratch[j];
            slot->rx_head = next;
        }

        /* One wake per event, not per byte — the interrupt-load win. */
        if (Size > 0 && slot->cb)
            slot->cb(UART_IRQ_RX, slot->cb_ctx);

        /* RxState was set READY by the HAL before this callback. */
        HAL_UARTEx_ReceiveToIdle_IT(&slot->huart, slot->rx_scratch,
                                    UART_RX_SCRATCH_SZ);
        return;
    }
}

/* On ORE/overrun (or framing/parity error) the HAL aborts the
 * ReceiveToIdle transfer and sets RxState READY WITHOUT an RxEvent
 * callback, which would leave RX permanently dead. Re-arm so RX
 * self-heals; bytes lost in the overrun are the caller's protocol to
 * recover (same resilience as the ring-full drop above). */
void HAL_UART_ErrorCallback(UART_HandleTypeDef *huart)
{
    for (int i = 0; i < UART_SLOT_COUNT; ++i)
    {
        uart_slot_t *slot = &uart_slots[i];
        if (!slot->in_use || slot->console_owned || &slot->huart != huart)
            continue;
        HAL_UARTEx_ReceiveToIdle_IT(&slot->huart, slot->rx_scratch,
                                    UART_RX_SCRATCH_SZ);
        return;
    }
}

void HAL_UART_TxCpltCallback(UART_HandleTypeDef *huart)
{
    for (int i = 0; i < UART_SLOT_COUNT; ++i)
    {
        uart_slot_t *slot = &uart_slots[i];
        if (!slot->in_use || &slot->huart != huart)
            continue;

        slot->tx_tail = (uint16_t)((slot->tx_tail + slot->tx_in_flight) % UART_TX_RING_SZ);
        slot->tx_in_flight = 0;

        uart_arm_tx(slot);

        if (!slot->tx_busy && slot->cb)
            slot->cb(UART_IRQ_TX_DONE, slot->cb_ctx);
        return;
    }
}

void uart_dispatch_by_slot(uint8_t slot_index)
{
    if (slot_index >= UART_SLOT_COUNT)
        return;
    uart_slot_t *slot = &uart_slots[slot_index];
    if (!slot->in_use)
        return;
    HAL_UART_IRQHandler(&slot->huart);
}
