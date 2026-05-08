#pragma once

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

// clock.c
void SystemClock_Config(void);

unsigned long long monotonic_now(void);
unsigned long long monotonic_freq(void);
void delay_us(uint32_t delay_us);
