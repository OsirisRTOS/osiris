#include "export.h"
#include "gpio.h"
#include "stm32l4xx.h"
#include "stm32l4xx_hal_can.h"
#include "stm32l4xx_hal_rcc.h"

#include <stm32l4xx_hal.h>
#include <stdbool.h>
#include <string.h>

#define CAN_SLOT_COUNT 2

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

#define CAN_RX_BUF_SIZE 128
#define CAN_TX_TIMEOUT_ITERS 1000u

typedef struct
{
    volatile can_frame_t frames[CAN_RX_BUF_SIZE];
    volatile uint8_t head;
    volatile uint8_t tail;
    volatile uint8_t count;
} can_rx_buf_t;

static CAN_HandleTypeDef s_handles[CAN_SLOT_COUNT];
static can_rx_buf_t s_rx[CAN_SLOT_COUNT];

static struct
{
    uint8_t rx0_irqn;
    uint8_t rx1_irqn;
} s_irqn[CAN_SLOT_COUNT];

static uint32_t s_tx_attempts[CAN_SLOT_COUNT];
static uint32_t s_tx_hal_fails[CAN_SLOT_COUNT];
static uint32_t s_tx_mbx_timeouts[CAN_SLOT_COUNT];
static uint32_t s_rx_frames[CAN_SLOT_COUNT];
static uint32_t s_rx_frames_fifo0[CAN_SLOT_COUNT];
static uint32_t s_rx_frames_fifo1[CAN_SLOT_COUNT];
static uint32_t s_rx_irqs[CAN_SLOT_COUNT];
static uint32_t s_rx_drops[CAN_SLOT_COUNT];
static uint32_t s_rx_hw_ovr[CAN_SLOT_COUNT];
static uint32_t s_rx_hw_ovr_fifo0[CAN_SLOT_COUNT];
static uint32_t s_rx_hw_ovr_fifo1[CAN_SLOT_COUNT];
static uint32_t s_rx_peak_fmp;
static uint32_t s_rx_get_fails[CAN_SLOT_COUNT];

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

/* Pick BRP/TS1/TS2 aiming for 75 % sample point and SJW = 1. */
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

    GPIO_InitTypeDef rx_gpio = {
        .Pin = rx_pin,
        .Mode = GPIO_MODE_AF_PP,
        .Pull = GPIO_PULLUP,
        .Speed = GPIO_SPEED_FREQ_VERY_HIGH,
        .Alternate = cfg->rx.af,
    };
    HAL_GPIO_Init(rx_port, &rx_gpio);

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
    HAL_NVIC_SetPriority((IRQn_Type)cfg->rx1_irqn, cfg->rx1_priority, 0);
    HAL_NVIC_EnableIRQ((IRQn_Type)cfg->rx1_irqn);
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

    s_irqn[cfg->index].rx0_irqn = cfg->rx0_irqn;
    s_irqn[cfg->index].rx1_irqn = cfg->rx1_irqn;

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
    h->Init.AutoRetransmission = ENABLE;
    h->Init.TimeTriggeredMode = DISABLE;
    h->Init.AutoWakeUp = DISABLE;
    /* RFLM=1: keep the oldest 3 frames on overflow; FOVR counts drops. */
    h->Init.ReceiveFifoLocked = ENABLE;
    h->Init.TransmitFifoPriority = ENABLE;

    if (HAL_CAN_Init(h) != HAL_OK)
    {
        return CAN_ERR_HAL_INIT;
    }

    /* Bus stays in HAL_CAN_STATE_READY so the caller can install filters
     * before HAL_CAN_Start (UM1884 §9.2.1 step order). */
    return 0;
}

int can_start(uint8_t slot)
{
    if (slot >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }
    CAN_HandleTypeDef *h = &s_handles[slot];
    if (h->Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    if (HAL_CAN_Start(h) != HAL_OK)
    {
        return CAN_ERR_HAL_START;
    }

    if (HAL_CAN_ActivateNotification(h,
                                     CAN_IT_RX_FIFO0_MSG_PENDING |
                                         CAN_IT_RX_FIFO1_MSG_PENDING) != HAL_OK)
    {
        return CAN_ERR_HAL_NOTIFY;
    }

    return 0;
}

int can_deinit(uint8_t slot)
{
    if (slot >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }

    HAL_NVIC_DisableIRQ((IRQn_Type)s_irqn[slot].rx0_irqn);
    HAL_NVIC_DisableIRQ((IRQn_Type)s_irqn[slot].rx1_irqn);
    HAL_CAN_DeInit(&s_handles[slot]);

    can_rx_buf_t *rx = &s_rx[slot];
    rx->head = 0;
    rx->tail = 0;
    rx->count = 0;

    return 0;
}

int can_transmit(uint8_t slot, const can_frame_t *frame)
{
    if (frame == NULL || slot >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }

    CAN_HandleTypeDef *h = &s_handles[slot];
    if (h->Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    s_tx_attempts[slot]++;

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

    /* Spin outside the critical section so RX keeps draining. */
    uint32_t timeout = CAN_TX_TIMEOUT_ITERS;
    while (HAL_CAN_GetTxMailboxesFreeLevel(h) == 0)
    {
        if (--timeout == 0)
        {
            s_tx_mbx_timeouts[slot]++;
            return CAN_ERR_TX_TIMEOUT;
        }
    }

    __disable_irq();
    uint32_t mailbox = 0;
    HAL_StatusTypeDef rc = HAL_CAN_AddTxMessage(h, &hdr, (uint8_t *)frame->data, &mailbox);
    __enable_irq();
    if (rc != HAL_OK)
    {
        s_tx_hal_fails[slot]++;
        return CAN_ERR_HAL_TX;
    }
    return 0;
}

int can_receive(uint8_t slot, can_frame_t *out)
{
    if (out == NULL || slot >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }

    if (s_handles[slot].Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    can_rx_buf_t *rx = &s_rx[slot];
    if (rx->count == 0)
    {
        return 0;
    }

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

static void drain_fifo(CAN_HandleTypeDef *hcan, uint8_t slot_idx, uint32_t fifo)
{
    s_rx_irqs[slot_idx]++;

    const bool is_fifo0 = (fifo == CAN_RX_FIFO0);
    volatile uint32_t *rfr =
        is_fifo0 ? &hcan->Instance->RF0R : &hcan->Instance->RF1R;
    const uint32_t fovr_flag = is_fifo0 ? CAN_RF0R_FOVR0 : CAN_RF1R_FOVR1;
    const uint32_t fmp_flag = is_fifo0 ? CAN_RF0R_FMP0 : CAN_RF1R_FMP1;
    const int kind = is_fifo0 ? CAN_IRQ_RX0 : CAN_IRQ_RX1;

    const uint32_t fmp_now = (*rfr & fmp_flag);
    if (fmp_now > s_rx_peak_fmp)
    {
        s_rx_peak_fmp = fmp_now;
    }

    if ((*rfr & fovr_flag) != 0u)
    {
        s_rx_hw_ovr[slot_idx]++;
        if (is_fifo0)
        {
            s_rx_hw_ovr_fifo0[slot_idx]++;
        }
        else
        {
            s_rx_hw_ovr_fifo1[slot_idx]++;
        }
        const uint32_t fov_clear = is_fifo0 ? CAN_FLAG_FOV0 : CAN_FLAG_FOV1;
        __HAL_CAN_CLEAR_FLAG(hcan, fov_clear);
    }

    can_rx_buf_t *rx = &s_rx[slot_idx];
    bool any_frame = false;

    while ((*rfr & fmp_flag) != 0u)
    {
        CAN_RxHeaderTypeDef hdr;

        volatile can_frame_t *slot = &rx->frames[rx->tail];
        if (HAL_CAN_GetRxMessage(hcan, fifo, &hdr, (void*)slot->data) != HAL_OK)
        {
            s_rx_get_fails[slot_idx]++;
            break;
        }

        s_rx_frames[slot_idx]++;
        if (is_fifo0)
        {
            s_rx_frames_fifo0[slot_idx]++;
        }
        else
        {
            s_rx_frames_fifo1[slot_idx]++;
        }

        if (rx->count >= CAN_RX_BUF_SIZE)
        {
            s_rx_drops[slot_idx]++;
            continue;
        }

        uint8_t len = (hdr.DLC > 8) ? 8 : (uint8_t)hdr.DLC;

        slot->id = (hdr.IDE == CAN_ID_EXT) ? hdr.ExtId : hdr.StdId;
        slot->len = len;
        slot->is_extended = (hdr.IDE == CAN_ID_EXT);
        /* slot->data has already been copied */

        rx->tail = (rx->tail + 1u) % CAN_RX_BUF_SIZE;
        rx->count++;
        any_frame = true;
    }

    if (any_frame)
    {
        can_irq_handler_fn fn = s_irq_slot[slot_idx].fn;
        if (fn != NULL)
        {
            fn(kind, s_irq_slot[slot_idx].ctx);
        }
    }
}

void HAL_CAN_RxFifo0MsgPendingCallback(CAN_HandleTypeDef *hcan)
{
    for (uint8_t i = 0; i < CAN_SLOT_COUNT; ++i)
    {
        if (&s_handles[i] == hcan)
        {
            drain_fifo(hcan, i, CAN_RX_FIFO0);
            return;
        }
    }
}

void HAL_CAN_RxFifo1MsgPendingCallback(CAN_HandleTypeDef *hcan)
{
    for (uint8_t i = 0; i < CAN_SLOT_COUNT; ++i)
    {
        if (&s_handles[i] == hcan)
        {
            drain_fifo(hcan, i, CAN_RX_FIFO1);
            return;
        }
    }
}

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

/* IDMASK encoding (RM0432 §55.7.4): EXID at <<3, STID at <<21, IDE at bit 2. */
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

int can_configure_filter(uint8_t slot, const can_filter_t *filter)
{
    if (filter == NULL || slot >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }
    if (s_handles[slot].Instance == NULL)
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

    return HAL_CAN_ConfigFilter(&s_handles[slot], &hal_f) == HAL_OK
               ? 0
               : CAN_ERR_HAL_FILTER;
}

uint32_t can_last_error(uint8_t slot)
{
    if (slot >= CAN_SLOT_COUNT)
    {
        return 0;
    }
    CAN_TypeDef *inst = s_handles[slot].Instance;
    return inst ? inst->ESR : 0;
}

int can_recover(uint8_t slot)
{
    if (slot >= CAN_SLOT_COUNT)
    {
        return CAN_ERR_INVALID_ARG;
    }
    CAN_HandleTypeDef *h = &s_handles[slot];
    if (h->Instance == NULL)
    {
        return CAN_ERR_NOT_INIT;
    }

    /* Drop scheduled TX so they don't refire after ABOM recovery. */
    __disable_irq();
    HAL_CAN_AbortTxRequest(h, CAN_TX_MAILBOX0 | CAN_TX_MAILBOX1 | CAN_TX_MAILBOX2);
    __enable_irq();
    return 0;
}

void can_diag(uint8_t slot, can_diag_t *out)
{
    if (out == NULL || slot >= CAN_SLOT_COUNT)
    {
        return;
    }
    CAN_TypeDef *inst = s_handles[slot].Instance;
    out->esr = inst ? inst->ESR : 0;
    out->tsr = inst ? inst->TSR : 0;
    out->msr = inst ? inst->MSR : 0;
    out->mcr = inst ? inst->MCR : 0;
    out->btr = inst ? inst->BTR : 0;
    out->tx_attempts = s_tx_attempts[slot];
    out->tx_hal_fails = s_tx_hal_fails[slot];
    out->tx_mbx_timeouts = s_tx_mbx_timeouts[slot];
    out->rx_irqs = s_rx_irqs[slot];
    out->rx_frames = s_rx_frames[slot];
    out->rx_frames_fifo0 = s_rx_frames_fifo0[slot];
    out->rx_frames_fifo1 = s_rx_frames_fifo1[slot];
    out->rx_drops = s_rx_drops[slot];
    out->rx_hw_ovr = s_rx_hw_ovr[slot];
    out->rx_hw_ovr_fifo0 = s_rx_hw_ovr_fifo0[slot];
    out->rx_hw_ovr_fifo1 = s_rx_hw_ovr_fifo1[slot];
    out->rx_peak_fmp = s_rx_peak_fmp;
    out->rx_get_fails = s_rx_get_fails[slot];
}
