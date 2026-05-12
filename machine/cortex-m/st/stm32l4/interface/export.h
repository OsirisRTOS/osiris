#pragma once
#include "stm32l4xx_hal_def.h"
#include "stm32l4xx_hal_flash.h"
#include <stdbool.h>
#include <stdint.h>

// bindgen-export: HAL_FLASH_ERROR_.*
// bindgen-export: FLASH_FLAG_.*

#define __FLASH_ERRORS                                                         \
  (FLASH_FLAG_SR_ERRORS | FLASH_FLAG_ALL_ERRORS | HAL_OK | HAL_BUSY |          \
   HAL_TIMEOUT)

#define FLASH_OK 0
_Static_assert((FLASH_OK & __FLASH_ERRORS) == 0);
_Static_assert(FLASH_OK == FLASH_ERROR_NONE);
#define ERR_FLASH_DOUBLE_UNLOCK (1 << 10)
_Static_assert((ERR_FLASH_DOUBLE_UNLOCK & __FLASH_ERRORS) == 0);
#define ERR_FLASH_NOT_UNLOCKED (1 << 11)
_Static_assert((ERR_FLASH_NOT_UNLOCKED & __FLASH_ERRORS) == 0);
#define ERR_FLASH_BUSY (1 << 12)
_Static_assert((ERR_FLASH_BUSY & __FLASH_ERRORS) == 0);
#define ERR_FLASH_ILLEGAL (1 << 13)
_Static_assert((ERR_FLASH_ILLEGAL & __FLASH_ERRORS) == 0);
#define ERR_FLASH_INVALID_PAGE (1 << 16)
_Static_assert((ERR_FLASH_INVALID_PAGE & __FLASH_ERRORS) == 0);
#define ERR_FLASH_INVALID_BANK (1 << 18)
_Static_assert((ERR_FLASH_INVALID_BANK & __FLASH_ERRORS) == 0);
#define ERR_FLASH_TIMEOUT (1 << 19)
_Static_assert((ERR_FLASH_TIMEOUT & __FLASH_ERRORS) == 0);

// bindgen-export: FLASH_OK
// bindgen-export: ERR_FLASH_.*

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
	uint8_t mode;
	uint8_t tx_open_drain;
	uint8_t reserved;
} can_bus_cfg_t;

typedef struct
{
	uint32_t id;
	uint8_t data[8];
	uint8_t len;
	uint8_t is_extended;
	uint16_t reserved;
} can_frame_t;

typedef struct
{
	uint32_t id;
	uint32_t mask;
	uint8_t bank;
	uint8_t extended;
	uint8_t fifo;
	uint8_t reserved;
} can_filter_t;

enum can_irq_kind
{
	CAN_IRQ_TX  = 0,
	CAN_IRQ_RX0 = 1,
	CAN_IRQ_RX1 = 2,
	CAN_IRQ_SCE = 3,
};

typedef void (*can_irq_handler_fn)(int kind, void *ctx);

int can_init(const can_bus_cfg_t *cfg);
int can_start(uint8_t slot);
int can_deinit(uint8_t slot);
int can_transmit(uint8_t slot, const can_frame_t *frame);
int can_receive(uint8_t slot, can_frame_t *out);
int can_configure_filter(uint8_t slot, const can_filter_t *filter);
uint32_t can_last_error(uint8_t slot);
int can_recover(uint8_t slot);
int can_set_irq_handler(uint8_t slot, can_irq_handler_fn handler, void *ctx);

typedef struct
{
	uint32_t esr;
	uint32_t tsr;
	uint32_t msr;
	uint32_t mcr;
	uint32_t btr;
	uint32_t tx_attempts;
	uint32_t tx_hal_fails;
	uint32_t tx_mbx_timeouts;
	uint32_t rx_irqs;
	uint32_t rx_frames;
	uint32_t rx_frames_fifo0;
	uint32_t rx_frames_fifo1;
	uint32_t rx_drops;
	uint32_t rx_hw_ovr;
	uint32_t rx_hw_ovr_fifo0;
	uint32_t rx_hw_ovr_fifo1;
	uint32_t rx_peak_fmp;
	uint32_t rx_get_fails;
} can_diag_t;
void can_diag(uint8_t slot, can_diag_t *out);

void can_isr(uint8_t index);

// sched.c
void reschedule(void);

// instru.c
void dwt_init(void);
void dwt_reset(void);
long dwt_read(void);
float dwt_read_ns(void);
float dwt_cycles_to_ns(long cycles);

// flash.c

// True if the flash controller's BSY bit is set.
bool flash_is_busy(void);

// Total flash size in bytes.
int flash_size(void);

// True if the device is configured for dual-bank mode (affects page size
// and how page numbers map to banks).
bool flash_is_dual_bank(void);

// Page size in bytes: 4 KiB in dual-bank mode, 8 KiB in single-bank mode.
int flash_page_size(void);

// Total number of flash pages.
int flash_page_count(void);

// Block until the last flash operation completes, or until timeout_ms
// milliseconds elapse.
uint32_t flash_wait_for_last_operation(uint32_t timeout_ms);

// Unlock the flash control register for erase/program. Fails with
// ERR_FLASH_BUSY if an operation is in flight, or ERR_FLASH_DOUBLE_UNLOCK if
// the flash is already unlocked.
uint32_t flash_unlock(void);

// Re-lock the flash control register. Caller must ensure no operation is in
// flight; returns ERR_FLASH_BUSY otherwise.
uint32_t flash_lock(void);

// Erase the flash page identified by `page_index` (a page number, not an
// address). Bank is derived automatically in dual-bank mode. timeout_ms
// bounds the wait for the erase to finish. Returns FLASH_OK, ERR_FLASH_BUSY,
// ERR_FLASH_INVALID_PAGE, or a HAL_FLASH_ERROR_* code.
uint32_t flash_erase(uint32_t page_index, uint32_t timeout_ms);

// Program `length` doublewords (uint64_t) starting at `start_address`.
// `data` must point to at least `length` elements. Flash must be unlocked
// and the target region must already be erased.
//
// timeout_ms is the total budget for the whole sequence, not per doubleword;
// each iteration is given the remaining time. Returns FLASH_OK,
// ERR_FLASH_BUSY, ERR_FLASH_TIMEOUT, or a HAL_FLASH_ERROR_* code.
uint32_t flash_program(uint32_t start_address, const uint64_t *data,
                       uint32_t length, uint32_t timeout_ms);

// clock.c
void SystemClock_Config(void);

unsigned long long monotonic_now(void);
unsigned long long monotonic_freq(void);
void delay_us(uint32_t delay_us);
void do_tick(void);
