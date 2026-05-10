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

// Erase one flash page by global page number (bank is derived automatically
// in dual-bank mode). timeout_ms bounds the wait for the erase to finish.
// Returns FLASH_OK, ERR_FLASH_BUSY, ERR_FLASH_INVALID_PAGE, or a
// HAL_FLASH_ERROR_* code.
uint32_t flash_erase(uint32_t page, uint32_t timeout_ms);

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
