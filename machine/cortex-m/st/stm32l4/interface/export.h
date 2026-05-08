#pragma once

#include <stdint.h>

// lib.c
unsigned long long systick_freq(void);
void init_hal(void);
__attribute__((noreturn)) void system_reset(void);

// uart.c
int init_debug_uart(void);
int write_debug_uart(const char *buf, int len);

// spi.c
typedef struct
{
	uintptr_t port;
	uint8_t pin;
	uint8_t af;
	uint16_t reserved;
} spi_pin_cfg_t;

typedef struct
{
	uintptr_t port;
	uint8_t pin;
	uint8_t active_low;
	uint16_t reserved;
} spi_cs_cfg_t;

typedef struct
{
	uintptr_t instance;
	spi_pin_cfg_t sck;
	spi_pin_cfg_t miso;
	spi_pin_cfg_t mosi;
} spi_bus_cfg_t;

typedef struct
{
	uintptr_t instance;
	uint32_t max_hz;
	uint8_t cpol;
	uint8_t cpha;
	uint8_t bits_per_word;
	uint8_t bit_order;
	uint32_t cs_setup_delay_us;
	uint32_t cs_hold_delay_us;
	uint32_t cs_inactive_delay_us;
	spi_cs_cfg_t cs;
	spi_cs_cfg_t enable;
} spi_device_cfg_t;

struct spi_transfer {
	const void *tx_words;
	void *rx_words;
	int word_count;
};

void* spi_init(const spi_bus_cfg_t *bus_cfg);
int spi_transfer(void *bus, const spi_device_cfg_t *dev_cfg, const struct spi_transfer *transfer);
int spi_deinit(void *bus);

int spi_init_device(const spi_device_cfg_t *dev_cfg);
int spi_deinit_device(const spi_device_cfg_t *dev_cfg);

// i2c.c
typedef struct
{
	uintptr_t port;
	uint8_t pin;
	uint8_t af;
	uint16_t reserved;
} i2c_pin_cfg_t;

typedef struct
{
	uintptr_t instance;
	uint32_t hz;
	uint32_t timingr;
	i2c_pin_cfg_t scl;
	i2c_pin_cfg_t sda;
} i2c_bus_cfg_t;

typedef struct
{
	uintptr_t port;
	uint8_t pin;
	uint8_t active_low;
	uint16_t reserved;
} i2c_gpio_cfg_t;

typedef struct
{
	uintptr_t instance;
	uint16_t address;
	uint16_t reserved;
	i2c_gpio_cfg_t enable;
} i2c_device_cfg_t;

struct i2c_transfer {
	const uint8_t *tx;
	uint8_t *rx;
	int tx_len;
	int rx_len;
};

void *i2c_init(const i2c_bus_cfg_t *bus_cfg);
int i2c_write(void *bus, const i2c_device_cfg_t *dev_cfg, struct i2c_transfer *transfer);
int i2c_read(void *bus, const i2c_device_cfg_t *dev_cfg, struct i2c_transfer *transfer);
int i2c_write_read(void *bus, const i2c_device_cfg_t *dev_cfg, const struct i2c_transfer *transfer);
int i2c_deinit(void *bus);

int i2c_init_device(const i2c_device_cfg_t *dev_cfg);
int i2c_deinit_device(const i2c_device_cfg_t *dev_cfg);

// can.c
typedef struct
{
	uintptr_t port;
	uint8_t pin;
	uint8_t af;
	uint16_t reserved;
} can_pin_cfg_t;

/* Operating mode selector passed to can_init. NORMAL is the everyday
   setting; LOOPBACK short-circuits TX back into RX inside the peripheral
   for diagnostics (no transceiver / bus required). */
enum can_mode
{
	CAN_MODE_NORMAL_ = 0,
	CAN_MODE_LOOPBACK_ = 1,
};

typedef struct
{
	uintptr_t instance;
	uint32_t bitrate_hz;
	can_pin_cfg_t rx;
	can_pin_cfg_t tx;
	uint8_t rx0_irqn;
	uint8_t rx0_priority;
	uint8_t rx1_irqn;
	uint8_t rx1_priority;
	uint8_t index;
	uint8_t mode;     /* one of enum can_mode */
	/* Non-zero ⇒ TX pin uses open-drain. Sourced from the DT pin node's
	   `drive-open-drain` boolean. */
	uint8_t tx_open_drain;
	/* Non-zero ⇒ peripheral retries on TX error (NART=0). Zero ⇒ one-shot:
	   a frame that fails arbitration or fails to be ACKed is dropped, not
	   retried. One-shot keeps an un-ACKable frame from re-firing TEC into
	   bus-off when an upper layer (CSP, etc.) already does its own retry. */
	uint8_t auto_retransmit;
	/* TX mailbox-free busy-loop limit, in iterations. Caller-controlled;
	   only read by `can_transmit`. Other entry points may pass 0. */
	uint32_t tx_timeout_iters;
} can_bus_cfg_t;

typedef struct
{
	uint32_t id;
	uint8_t data[8];
	uint8_t len;
	uint8_t is_extended;
	uint16_t reserved;
} can_frame_t;

/* 32-bit ID-mask filter. bxCAN has 14 banks on L4R5; pick one via `bank`.
   For extended frames, `id` / `mask` are interpreted as 29-bit extended IDs;
   for standard frames, as 11-bit standard IDs. `fifo` selects RX FIFO 0 or 1. */
typedef struct
{
	uint32_t id;
	uint32_t mask;
	uint8_t bank;
	uint8_t extended;
	uint8_t fifo;
	uint8_t reserved;
} can_filter_t;

/* Per-slot ISR-context callback kind. Currently only RX0 is enabled;
   TX/RX1/SCE reserved for future expansion. */
enum can_irq_kind
{
	CAN_IRQ_TX  = 0,
	CAN_IRQ_RX0 = 1,
	CAN_IRQ_RX1 = 2,
	CAN_IRQ_SCE = 3,
};

/* ISR-context callback invoked after every successful ring push. The
   hook runs in CAN1_RX0 IRQ context — keep it brief (atomic store,
   scheduler kick). `ctx` is the opaque pointer last passed to
   `can_set_irq_handler`; the kernel never dereferences it. The frames
   themselves are drained via `can_receive` from thread context. */
typedef void (*can_irq_handler_fn)(int kind, void *ctx);

int can_init(const can_bus_cfg_t *cfg);
int can_deinit(const can_bus_cfg_t *cfg);
int can_transmit(const can_bus_cfg_t *cfg, const can_frame_t *frame);
/* Pull one frame from the SW ring. Returns 1 on success (writes `*out`),
   0 if the ring is empty, negative on error. */
int can_receive(const can_bus_cfg_t *cfg, can_frame_t *out);
int can_configure_filter(const can_bus_cfg_t *cfg, const can_filter_t *filter);
int can_disable_filter(const can_bus_cfg_t *cfg, uint8_t bank);
/* Returns CAN_ESR (error status register) read directly from hardware.
   Layout (RM0432 §55.9 CAN_ESR):
     [31:24] REC, [23:16] TEC, [6:4] LEC, [2] BOFF, [1] EPVF, [0] EWGF */
uint32_t can_last_error(const can_bus_cfg_t *cfg);

/* Abort all three TX mailboxes. Call after detecting bus-off so a
   SCHEDULED un-ACKable frame doesn't re-flood the bus once ABOM=1
   finishes the 128*11-bit recovery (RM0432 §55.7.6). */
int can_recover(const can_bus_cfg_t *cfg);

/* Install (or clear, with handler=NULL) the per-slot ISR-context
   callback fired after each successful ring push. Runs in IRQ context
   — keep it brief (atomic store, scheduler kick); thread-side work
   belongs in the consumer that calls `can_receive`. Returns 0 on
   success, negative on invalid slot. */
int can_set_irq_handler(uint8_t slot, can_irq_handler_fn handler, void *ctx);

typedef struct
{
	uint32_t esr;             /* CAN_ESR snapshot */
	uint32_t tsr;             /* CAN_TSR snapshot (TX mailbox status) */
	uint32_t msr;             /* CAN_MSR snapshot (master status) */
	uint32_t mcr;             /* CAN_MCR snapshot (master control) */
	uint32_t btr;             /* CAN_BTR snapshot (bit timing + SILM/LBKM) */
	uint32_t tx_attempts;     /* calls into can_transmit */
	uint32_t tx_hal_fails;    /* HAL_CAN_AddTxMessage != HAL_OK */
	uint32_t tx_mbx_timeouts; /* mailbox stayed busy → busy-loop timed out */
	uint32_t rx_irqs;         /* RX IRQs observed (FIFO0 only at present) */
	uint32_t rx_frames;       /* frames successfully pulled out of FIFO0 */
	uint32_t rx_drops;        /* frames dropped (SW ring full) */
	uint32_t rx_hw_ovr;       /* HW FOVR0: bxCAN FIFO overflowed (frames silently dropped before reaching us) */
} can_diag_t;
void can_diag(const can_bus_cfg_t *cfg, can_diag_t *out);

/* IRQ-vector entry point. Called by the kernel IRQ registry for the
   CAN1_RX0 vector; dispatches to `HAL_CAN_IRQHandler`. */
void can_isr(uint8_t index);

// sched.c
void reschedule(void);

// instru.c
void dwt_init(void);
void dwt_reset(void);
long dwt_read(void);
float dwt_read_ns(void);
float dwt_cycles_to_ns(long cycles);

// clock.c
void SystemClock_Config(void);

unsigned long long monotonic_now(void);
unsigned long long monotonic_freq(void);
void delay_us(uint32_t delay_us);
