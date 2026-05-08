#include "export.h"
#include "gpio.h"
#include "stm32l4xx.h"
#include "stm32l4xx_hal_can.h"
#include "stm32l4xx_hal_rcc.h"

#include <stm32l4xx_hal.h>
#include <stdbool.h>
#include <string.h>

/* Maximum bxCAN instances on any STM32L4 family member (CAN1, CAN2). The
   value is intentionally hardcoded here rather than DT-derived: arrays
   sized at compile time, no codegen header dependency. The DT still
   selects which slot each peripheral lives in via cfg->index. */
#define CAN_SLOT_COUNT 2

/* Negative return codes from can_*. Each value has exactly one meaning so
   the Rust HAL can map directly to an Error variant. Keep in sync with
   `osiris/machine/cortex-m/src/native/can.rs::Error` mapping. */
#define CAN_ERR_INVALID_ARG (-1)
#define CAN_ERR_NOT_INIT    (-2)
#define CAN_ERR_BITRATE     (-3)
#define CAN_ERR_CLOCK       (-4)
#define CAN_ERR_HAL_INIT    (-5)
#define CAN_ERR_HAL_FILTER  (-6)
#define CAN_ERR_HAL_START   (-7)
#define CAN_ERR_HAL_NOTIFY  (-8)
#define CAN_ERR_HAL_TX      (-9)
#define CAN_ERR_TX_TIMEOUT  (-10)

#define CAN_RX_BUF_SIZE 32

typedef struct
{
    volatile can_frame_t frames[CAN_RX_BUF_SIZE];
    volatile uint8_t head;
    volatile uint8_t tail;
    volatile uint8_t count;
} can_rx_buf_t;

static CAN_HandleTypeDef s_handles[CAN_SLOT_COUNT];
static can_rx_buf_t s_rx[CAN_SLOT_COUNT];

/* Diagnostic counters surfaced via can_diag(). */
static uint32_t s_tx_attempts[CAN_SLOT_COUNT];
static uint32_t s_tx_hal_fails[CAN_SLOT_COUNT];
static uint32_t s_tx_mbx_timeouts[CAN_SLOT_COUNT];
static uint32_t s_rx_frames[CAN_SLOT_COUNT];
static uint32_t s_rx_irqs[CAN_SLOT_COUNT];
static uint32_t s_rx_drops[CAN_SLOT_COUNT];
/* HW FIFO overrun (FOVR0): bxCAN dropped a frame before we could read it. */
static uint32_t s_rx_hw_ovr[CAN_SLOT_COUNT];

/* Per-slot ISR callback. The HAL never dereferences `ctx` — it just
   passes the value through to `fn` on every IRQ. The caller is
   responsible for keeping the pointee alive while registered. */
static struct
{
    can_irq_handler_fn fn;
    void *ctx;
} s_irq_slot[CAN_SLOT_COUNT];

static int can_enable_clock(CAN_TypeDef *instance)
{
#if defined(CAN1)
    if (instance == CAN1)
    {
        __HAL_RCC_CAN1_FORCE_RESET();
        __HAL_RCC_CAN1_RELEASE_RESET();
        __HAL_RCC_CAN1_CLK_ENABLE();
        return 0;
    }
#endif
#if defined(CAN2)
    if (instance == CAN2)
    {
        __HAL_RCC_CAN2_FORCE_RESET();
        __HAL_RCC_CAN2_RELEASE_RESET();
        __HAL_RCC_CAN2_CLK_ENABLE();
        return 0;
    }
#endif
    return -1;
}

/* Pick BRP/TS1/TS2 for a given PCLK and target bitrate, aiming for 75 %
   sample point and SJW = 1. Returns 0 on success, -1 if no preset divides
   cleanly (silent fallback would mis-baud the bus). */
static int can_bit_timing(uint32_t pclk_hz, uint32_t bitrate_hz,
                          uint32_t *out_prescaler,
                          uint32_t *out_ts1,
                          uint32_t *out_ts2)
{
    static const struct
    {
        uint8_t nbt;
        uint8_t ts1;
        uint8_t ts2;
    } presets[] = {
        {16, 12, 3},
        {12, 8, 3},
        {10, 7, 2},
        {8, 5, 2},
    };

    for (size_t i = 0; i < sizeof(presets) / sizeof(presets[0]); ++i)
    {
        uint32_t divisor = bitrate_hz * (uint32_t)presets[i].nbt;
        if (divisor == 0)
        {
            continue;
        }
        if (pclk_hz % divisor == 0)
        {
            *out_prescaler = pclk_hz / divisor;
            *out_ts1 = (uint32_t)(presets[i].ts1 - 1) << CAN_BTR_TS1_Pos;
            *out_ts2 = (uint32_t)(presets[i].ts2 - 1) << CAN_BTR_TS2_Pos;
            return 0;
        }
    }
    return -1;
}

static void can_msp_init(const can_bus_cfg_t *cfg)
{
    GPIO_TypeDef *rx_port = (GPIO_TypeDef *)cfg->rx.port;
    GPIO_TypeDef *tx_port = (GPIO_TypeDef *)cfg->tx.port;
    uint16_t rx_pin = (uint16_t)(1u << cfg->rx.pin);
    uint16_t tx_pin = (uint16_t)(1u << cfg->tx.pin);

    gpio_enable_clock(rx_port);
    gpio_enable_clock(tx_port);

    /* RX: AF push-pull input with pull-up (recessive level when bus idle). */
    GPIO_InitTypeDef rx_gpio = {
        .Pin = rx_pin,
        .Mode = GPIO_MODE_AF_PP,
        .Pull = GPIO_PULLUP,
        .Speed = GPIO_SPEED_FREQ_VERY_HIGH,
        .Alternate = cfg->rx.af,
    };
    HAL_GPIO_Init(rx_port, &rx_gpio);

    /* TX drive: push-pull when a transceiver sits between MCU and bus
       (normal case); open-drain when MCU pins run direct into a wired-AND
       bench setup without a transceiver. */
    GPIO_InitTypeDef tx_gpio = {
        .Pin = tx_pin,
        .Mode = cfg->tx_open_drain ? GPIO_MODE_AF_OD : GPIO_MODE_AF_PP,
        .Pull = GPIO_NOPULL,
        .Speed = GPIO_SPEED_FREQ_VERY_HIGH,
        .Alternate = cfg->tx.af,
    };
    HAL_GPIO_Init(tx_port, &tx_gpio);

    HAL_NVIC_SetPriority((IRQn_Type)cfg->rx0_irqn, cfg->rx0_priority, 0);
    HAL_NVIC_EnableIRQ((IRQn_Type)cfg->rx0_irqn);
}

int can_init(const can_bus_cfg_t *cfg)
{
    if (cfg == NULL || cfg->index >= CAN_SLOT_COUNT || cfg->bitrate_hz == 0)
    {
        return CAN_ERR_INVALID_ARG;
    }

    CAN_TypeDef *instance = (CAN_TypeDef *)cfg->instance;
    if (can_enable_clock(instance) != 0)
    {
        return CAN_ERR_CLOCK;
    }

    can_rx_buf_t *rx = &s_rx[cfg->index];
    rx->head = 0;
    rx->tail = 0;
    rx->count = 0;

    can_msp_init(cfg);

    uint32_t prescaler = 0;
    uint32_t ts1 = 0;
    uint32_t ts2 = 0;
    if (can_bit_timing(HAL_RCC_GetPCLK1Freq(), cfg->bitrate_hz, &prescaler, &ts1, &ts2) != 0)
    {
        return CAN_ERR_BITRATE;
    }

    CAN_HandleTypeDef *h = &s_handles[cfg->index];
    memset(h, 0, sizeof(*h));
    h->Instance = instance;
    h->Init.Prescaler = prescaler;
    h->Init.Mode = (cfg->mode == 1) ? CAN_MODE_SILENT_LOOPBACK : CAN_MODE_NORMAL;
    h->Init.SyncJumpWidth = CAN_SJW_1TQ;
    h->Init.TimeSeg1 = ts1;
    h->Init.TimeSeg2 = ts2;
    h->Init.AutoBusOff = ENABLE;
    /* NART=0: HW retransmits NAKed/error frames until success or arbitration
       loss (RM0432 §55.9.2 CAN_MCR.NART). Required when no upper-layer
       per-frame retry exists. */
    h->Init.AutoRetransmission = ENABLE;
    h->Init.TimeTriggeredMode = DISABLE;
    h->Init.AutoWakeUp = DISABLE;
    h->Init.ReceiveFifoLocked = ENABLE;
    h->Init.TransmitFifoPriority = ENABLE;

    if (HAL_CAN_Init(h) != HAL_OK)
    {
        return CAN_ERR_HAL_INIT;
    }

    /* No filters installed by default: bxCAN drops every frame until the
       caller adds at least one via can_configure_filter. Multiple banks
       are OR'd, so a leftover accept-all here would silently neutralise
       any restrictive user filter. */

    if (HAL_CAN_Start(h) != HAL_OK)
    {
        return CAN_ERR_HAL_START;
    }

    if (HAL_CAN_ActivateNotification(h, CAN_IT_RX_FIFO0_MSG_PENDING) != HAL_OK)
    {
        return CAN_ERR_HAL_NOTIFY;
    }

    return 0;
}

int can_deinit(const can_bus_cfg_t *cfg)
{
    if (cfg == NULL || cfg->index >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }

    HAL_NVIC_DisableIRQ((IRQn_Type)cfg->rx0_irqn);
    HAL_CAN_DeInit(&s_handles[cfg->index]);

    can_rx_buf_t *rx = &s_rx[cfg->index];
    rx->head = 0;
    rx->tail = 0;
    rx->count = 0;

    return 0;
}

int can_transmit(const can_bus_cfg_t *cfg, const can_frame_t *frame)
{
    if (cfg == NULL || frame == NULL || cfg->index >= CAN_SLOT_COUNT
        || cfg->tx_timeout_iters == 0)
    {
        return CAN_ERR_INVALID_ARG;
    }

    CAN_HandleTypeDef *h = &s_handles[cfg->index];
    if (h->Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    s_tx_attempts[cfg->index]++;

    CAN_TxHeaderTypeDef hdr = {
        .IDE = frame->is_extended ? CAN_ID_EXT : CAN_ID_STD,
        .RTR = CAN_RTR_DATA,
        .DLC = frame->len,
        .TransmitGlobalTime = DISABLE,
    };
    if (frame->is_extended)
    {
        hdr.ExtId = frame->id;
    }
    else
    {
        hdr.StdId = frame->id;
    }

    /* Spin outside the critical section so the RX ISR keeps draining FIFO0
       even if the bus is stuck for the full timeout. */
    uint32_t timeout = cfg->tx_timeout_iters;
    while (HAL_CAN_GetTxMailboxesFreeLevel(h) == 0)
    {
        if (--timeout == 0)
        {
            s_tx_mbx_timeouts[cfg->index]++;
            return CAN_ERR_TX_TIMEOUT;
        }
    }

    /* Serialize AddTxMessage: the HAL handle and PendSV both alias
       h->ErrorCode / h->State / mailbox bookkeeping. */
    __disable_irq();
    uint32_t mailbox = 0;
    HAL_StatusTypeDef rc = HAL_CAN_AddTxMessage(h, &hdr, (uint8_t *)frame->data, &mailbox);
    __enable_irq();
    if (rc != HAL_OK)
    {
        s_tx_hal_fails[cfg->index]++;
        return CAN_ERR_HAL_TX;
    }
    return 0;
}

int can_receive(const can_bus_cfg_t *cfg, can_frame_t *out)
{
    if (cfg == NULL || out == NULL || cfg->index >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }

    if (s_handles[cfg->index].Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    can_rx_buf_t *rx = &s_rx[cfg->index];
    if (rx->count == 0)
    {
        return 0;
    }

    /* Disable the CAN RX IRQ briefly so the ISR can't race the consumer. */
    __disable_irq();
    if (rx->count == 0)
    {
        __enable_irq();
        return 0;
    }

    const volatile can_frame_t *src = &rx->frames[rx->head];
    out->id = src->id;
    out->len = src->len;
    out->is_extended = src->is_extended;
    memcpy(out->data, (const void *)src->data, sizeof(out->data));

    rx->head = (rx->head + 1u) % CAN_RX_BUF_SIZE;
    rx->count--;
    __enable_irq();

    return 1;
}

/* Install (or clear, if `handler == NULL`) the per-slot ISR callback. The
   write is wrapped in __disable_irq so the RX ISR can't see a half-updated
   (fn, ctx) pair. */
int can_set_irq_handler(uint8_t slot, can_irq_handler_fn handler, void *ctx)
{
    if (slot >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }
    __disable_irq();
    s_irq_slot[slot].fn = handler;
    s_irq_slot[slot].ctx = ctx;
    __enable_irq();
    return 0;
}

/* HAL `__weak` override: drain every pending frame from FIFO0 per IRQ.
   Per-IRQ entry/exit cost (vector → trampoline → HAL prolog) is large
   relative to one frame at 1 Mbit/s, so doing one frame per IRQ overruns
   the 3-deep HW FIFO under bursts. */
void HAL_CAN_RxFifo0MsgPendingCallback(CAN_HandleTypeDef *hcan)
{
    for (uint8_t i = 0; i < CAN_SLOT_COUNT; ++i)
    {
        if (&s_handles[i] != hcan)
        {
            continue;
        }

        s_rx_irqs[i]++;

        /* RFLM=1 keeps the oldest 3 frames on overflow; FOVR0 set means
           a newer frame was silently dropped. Clear so the next overrun
           is detectable. */
        if ((hcan->Instance->RF0R & CAN_RF0R_FOVR0) != 0u)
        {
            s_rx_hw_ovr[i]++;
            __HAL_CAN_CLEAR_FLAG(hcan, CAN_FLAG_FOV0);
        }

        can_rx_buf_t *rx = &s_rx[i];
        bool any_frame = false;

        while ((hcan->Instance->RF0R & CAN_RF0R_FMP0) != 0u)
        {
            CAN_RxHeaderTypeDef hdr;
            uint8_t data[8];
            if (HAL_CAN_GetRxMessage(hcan, CAN_RX_FIFO0, &hdr, data) != HAL_OK)
            {
                break;
            }

            s_rx_frames[i]++;

            if (rx->count >= CAN_RX_BUF_SIZE)
            {
                s_rx_drops[i]++;
                continue;
            }

            /* DLC is a 4-bit field (0..=15); CAN 2.0 caps payload at 8
               bytes but a corrupt bus can deliver 9..=15. Clamp before
               any indexing so neither `data[8]` nor `slot->data[8]`
               overflows. */
            uint8_t len = (hdr.DLC > 8) ? 8 : (uint8_t)hdr.DLC;

            volatile can_frame_t *slot = &rx->frames[rx->tail];
            slot->id = (hdr.IDE == CAN_ID_EXT) ? hdr.ExtId : hdr.StdId;
            slot->len = len;
            slot->is_extended = (hdr.IDE == CAN_ID_EXT);
            memcpy((void *)slot->data, data, len);

            rx->tail = (rx->tail + 1u) % CAN_RX_BUF_SIZE;
            rx->count++;
            any_frame = true;
        }

        if (any_frame)
        {
            can_irq_handler_fn fn = s_irq_slot[i].fn;
            if (fn != NULL)
            {
                fn(CAN_IRQ_RX0, s_irq_slot[i].ctx);
            }
        }
        return;
    }
}

/* IRQ trampoline target — kernel IRQ-registry handler dispatches here. */
void can_isr(uint8_t index)
{
    if (index >= CAN_SLOT_COUNT)
    {
        return;
    }
    if (s_handles[index].Instance != NULL)
    {
        HAL_CAN_IRQHandler(&s_handles[index]);
    }
}

/* 32-bit IDMASK encoding (RM0432 §55.7.4): bits [31..3] hold the ID
   (STID[10:0] at [31:21], EXID[28:0] at [31:3]), bit [2] is IDE.
   Extended IDs land at <<3, standard at <<21. Setting IDE in the mask
   rejects the wrong frame kind. */
static void encode_filter(const can_filter_t *f, uint32_t *id_reg, uint32_t *mask_reg)
{
    if (f->extended)
    {
        *id_reg = (f->id << 3) | (1u << 2);
        *mask_reg = (f->mask << 3) | (1u << 2);
    }
    else
    {
        *id_reg = (f->id & 0x7FFu) << 21;
        *mask_reg = ((f->mask & 0x7FFu) << 21) | (1u << 2);
    }
}

int can_configure_filter(const can_bus_cfg_t *cfg, const can_filter_t *filter)
{
    if (cfg == NULL || filter == NULL || cfg->index >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }
    if (s_handles[cfg->index].Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    uint32_t id_reg = 0;
    uint32_t mask_reg = 0;
    encode_filter(filter, &id_reg, &mask_reg);

    CAN_FilterTypeDef hal_f = {
        .FilterBank = filter->bank,
        .FilterMode = CAN_FILTERMODE_IDMASK,
        .FilterScale = CAN_FILTERSCALE_32BIT,
        .FilterIdHigh = (uint16_t)((id_reg >> 16) & 0xFFFFu),
        .FilterIdLow = (uint16_t)(id_reg & 0xFFFFu),
        .FilterMaskIdHigh = (uint16_t)((mask_reg >> 16) & 0xFFFFu),
        .FilterMaskIdLow = (uint16_t)(mask_reg & 0xFFFFu),
        .FilterFIFOAssignment = (filter->fifo == 1) ? CAN_RX_FIFO1 : CAN_RX_FIFO0,
        .FilterActivation = CAN_FILTER_ENABLE,
        .SlaveStartFilterBank = 14,
    };

    return HAL_CAN_ConfigFilter(&s_handles[cfg->index], &hal_f) == HAL_OK
               ? 0
               : CAN_ERR_HAL_FILTER;
}

int can_disable_filter(const can_bus_cfg_t *cfg, uint8_t bank)
{
    if (cfg == NULL || cfg->index >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }
    if (s_handles[cfg->index].Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    CAN_FilterTypeDef hal_f = {
        .FilterBank = bank,
        .FilterActivation = CAN_FILTER_DISABLE,
        .SlaveStartFilterBank = 14,
    };
    return HAL_CAN_ConfigFilter(&s_handles[cfg->index], &hal_f) == HAL_OK
               ? 0
               : CAN_ERR_HAL_FILTER;
}

uint32_t can_last_error(const can_bus_cfg_t *cfg)
{
    if (cfg == NULL || cfg->index >= CAN_SLOT_COUNT)
    {
        return 0;
    }
    /* Read ESR directly — HAL_CAN_GetError only latches with error IRQs on,
       and we only enable CAN_IT_RX_FIFO0_MSG_PENDING. See RM0432 §55.9 (CAN_ESR). */
    CAN_TypeDef *inst = s_handles[cfg->index].Instance;
    if (inst == NULL)
    {
        return 0;
    }
    return inst->ESR;
}

int can_recover(const can_bus_cfg_t *cfg)
{
    if (cfg == NULL || cfg->index >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }
    CAN_HandleTypeDef *h = &s_handles[cfg->index];
    if (h->Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    /* TSR.ABRQ writes; safe in bus-off. ABOM=1 handles the 128*11-bit
       protocol recovery (RM0432 §55.7.6) — we just drop the SCHEDULED
       frames so they don't re-fire the moment hardware comes back. */
    __disable_irq();
    HAL_CAN_AbortTxRequest(h, CAN_TX_MAILBOX0 | CAN_TX_MAILBOX1 | CAN_TX_MAILBOX2);
    __enable_irq();
    return 0;
}

void can_diag(const can_bus_cfg_t *cfg, can_diag_t *out)
{
    if (cfg == NULL || out == NULL || cfg->index >= CAN_SLOT_COUNT)
    {
        return;
    }
    CAN_TypeDef *inst = s_handles[cfg->index].Instance;
    out->esr = inst ? inst->ESR : 0;
    out->tsr = inst ? inst->TSR : 0;
    out->msr = inst ? inst->MSR : 0;
    out->mcr = inst ? inst->MCR : 0;
    out->btr = inst ? inst->BTR : 0;
    out->tx_attempts = s_tx_attempts[cfg->index];
    out->tx_hal_fails = s_tx_hal_fails[cfg->index];
    out->tx_mbx_timeouts = s_tx_mbx_timeouts[cfg->index];
    out->rx_irqs = s_rx_irqs[cfg->index];
    out->rx_frames = s_rx_frames[cfg->index];
    out->rx_drops = s_rx_drops[cfg->index];
    out->rx_hw_ovr = s_rx_hw_ovr[cfg->index];
}
