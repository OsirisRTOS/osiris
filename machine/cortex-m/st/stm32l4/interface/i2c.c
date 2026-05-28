#include "lib.h"
#include "export.h"
#include "gpio.h"
#include "hal_api.h"
#include "stm32l4xx.h"
#include "stm32l4xx_hal_gpio.h"
#include "stm32l4xx_hal_rcc.h"

#include <stm32l4xx_hal.h>

#define I2C_SLOT_COUNT 4
#define I2C_RECOVERY_CLOCKS 9
#define I2C_RECOVERY_DELAY_US 5U
#define I2C_ENABLE_SETTLE_US 10000U

struct i2c_bus {
    uint8_t in_use;
    i2c_bus_cfg_t bus_cfg;
    I2C_HandleTypeDef hi2c;
};

static struct i2c_bus i2c_buses[I2C_SLOT_COUNT];

static int i2c_configure_hal(struct i2c_bus *bus)
{
    bus->hi2c.Instance = (I2C_TypeDef *)bus->bus_cfg.instance;
    bus->hi2c.Init.Timing = bus->bus_cfg.timingr;
    bus->hi2c.Init.AddressingMode = I2C_ADDRESSINGMODE_7BIT;
    bus->hi2c.State = HAL_I2C_STATE_RESET;

    if (HAL_I2C_Init(&bus->hi2c) != HAL_OK)
    {
        return -PosixError_EIO;
    }

    if (HAL_I2CEx_ConfigAnalogFilter(&bus->hi2c, I2C_ANALOGFILTER_ENABLE) != HAL_OK)
    {
        (void)HAL_I2C_DeInit(&bus->hi2c);
        return -PosixError_EIO;
    }

    if (HAL_I2CEx_ConfigDigitalFilter(&bus->hi2c, 0) != HAL_OK)
    {
        (void)HAL_I2C_DeInit(&bus->hi2c);
        return -PosixError_EIO;
    }

    return 0;
}

static int i2c_transfer_error(struct i2c_bus *bus, HAL_StatusTypeDef status)
{
    if (status == HAL_TIMEOUT)
    {
        return -PosixError_ETIMEDOUT;
    }

    if (status == HAL_BUSY)
    {
        return -PosixError_EBUSY;
    }

    uint32_t err = HAL_I2C_GetError(&bus->hi2c);
    if ((err & HAL_I2C_ERROR_TIMEOUT) != 0)
    {
        return -PosixError_ETIMEDOUT;
    }

    if ((err & HAL_I2C_ERROR_INVALID_PARAM) != 0)
    {
        return -PosixError_EINVAL;
    }

    if ((err & HAL_I2C_ERROR_AF) != 0)
    {
        return -PosixError_ENXIO;
    }

    if ((err & HAL_I2C_ERROR_ARLO) != 0)
    {
        return -PosixError_EAGAIN;
    }

    if ((err & HAL_I2C_ERROR_BERR) != 0)
    {
        return -PosixError_EPROTO;
    }

    if ((err & HAL_I2C_ERROR_OVR) != 0)
    {
        return -PosixError_EOVERFLOW;
    }

    if ((err & HAL_I2C_ERROR_SIZE) != 0)
    {
        return -PosixError_EMSGSIZE;
    }

    if ((err & (HAL_I2C_ERROR_DMA | HAL_I2C_ERROR_DMA_PARAM)) != 0)
    {
        return -PosixError_EIO;
    }

#if defined(HAL_I2C_ERROR_INVALID_CALLBACK)
    if ((err & HAL_I2C_ERROR_INVALID_CALLBACK) != 0)
    {
        return -PosixError_EINVAL;
    }
#endif

    return -PosixError_EIO;
}

static int i2c_enable_clock(I2C_TypeDef *instance)
{
#if defined(I2C1)
    if (instance == I2C1)
    {
        __HAL_RCC_I2C1_CLK_ENABLE();
        return 0;
    }
#endif
#if defined(I2C2)
    if (instance == I2C2)
    {
        __HAL_RCC_I2C2_CLK_ENABLE();
        return 0;
    }
#endif
#if defined(I2C3)
    if (instance == I2C3)
    {
        __HAL_RCC_I2C3_CLK_ENABLE();
        return 0;
    }
#endif
#if defined(I2C4)
    if (instance == I2C4)
    {
        __HAL_RCC_I2C4_CLK_ENABLE();
        return 0;
    }
#endif

    return -PosixError_ENODEV;
}

static int i2c_disable_clock(I2C_TypeDef *instance)
{
#if defined(I2C1)
    if (instance == I2C1)
    {
        __HAL_RCC_I2C1_CLK_DISABLE();
        return 0;
    }
#endif
#if defined(I2C2)
    if (instance == I2C2)
    {
        __HAL_RCC_I2C2_CLK_DISABLE();
        return 0;
    }
#endif
#if defined(I2C3)
    if (instance == I2C3)
    {
        __HAL_RCC_I2C3_CLK_DISABLE();
        return 0;
    }
#endif
#if defined(I2C4)
    if (instance == I2C4)
    {
        __HAL_RCC_I2C4_CLK_DISABLE();
        return 0;
    }
#endif

    return -PosixError_ENODEV;
}

int i2c_bus_recovery_needed(void *bus)
{
    if (bus == 0)
    {
        return -PosixError_EINVAL;
    }

    struct i2c_bus *i2c_bus = (struct i2c_bus *)bus;
    if (!i2c_bus->in_use)
    {
        return -PosixError_EINVAL;
    }

    GPIO_TypeDef *scl_port = (GPIO_TypeDef *)i2c_bus->bus_cfg.scl.port;
    GPIO_TypeDef *sda_port = (GPIO_TypeDef *)i2c_bus->bus_cfg.sda.port;
    uint16_t scl_pin = (uint16_t)(1u << i2c_bus->bus_cfg.scl.pin);
    uint16_t sda_pin = (uint16_t)(1u << i2c_bus->bus_cfg.sda.pin);

    if (__HAL_I2C_GET_FLAG(&i2c_bus->hi2c, I2C_FLAG_BUSY) ||
        HAL_GPIO_ReadPin(scl_port, scl_pin) == GPIO_PIN_RESET ||
        HAL_GPIO_ReadPin(sda_port, sda_pin) == GPIO_PIN_RESET)
    {
        return 1;
    }

    return 0;
}

void *i2c_init(const i2c_bus_cfg_t *bus_cfg)
{
    if (!bus_cfg)
    {
        return 0;
    }

    struct i2c_bus *bus = 0;

    for (int i = 0; i < I2C_SLOT_COUNT; ++i) {
        if (!i2c_buses[i].in_use)
        {
            bus = &i2c_buses[i];
            break;
        }
    }

    if (bus == 0)
    {
        return 0;
    }

    GPIO_TypeDef *scl_port = (GPIO_TypeDef *)bus_cfg->scl.port;
    GPIO_TypeDef *sda_port = (GPIO_TypeDef *)bus_cfg->sda.port;
    uint16_t scl_pin = (uint16_t)(1u << bus_cfg->scl.pin);
    uint16_t sda_pin = (uint16_t)(1u << bus_cfg->sda.pin);

    if (i2c_enable_clock((I2C_TypeDef *)bus_cfg->instance) != 0)
    {
        return 0;
    }

    gpio_enable_clock(scl_port);
    gpio_enable_clock(sda_port);
    gpio_init_af_od(scl_port, scl_pin, bus_cfg->scl.af);
    gpio_init_af_od(sda_port, sda_pin, bus_cfg->sda.af);

    bus->bus_cfg = *bus_cfg;
    if (i2c_configure_hal(bus) != 0)
    {
        (void)i2c_disable_clock((I2C_TypeDef *)bus_cfg->instance);
        return 0;
    }

    bus->in_use = 1;
    return (void *)bus;
}

int i2c_recover_bus(void *bus)
{
    if (bus == 0)
    {
        return -PosixError_EINVAL;
    }

    struct i2c_bus *i2c_bus = (struct i2c_bus *)bus;
    if (!i2c_bus->in_use)
    {
        return -PosixError_EINVAL;
    }

    GPIO_TypeDef *scl_port = (GPIO_TypeDef *)i2c_bus->bus_cfg.scl.port;
    GPIO_TypeDef *sda_port = (GPIO_TypeDef *)i2c_bus->bus_cfg.sda.port;
    uint16_t scl_pin = (uint16_t)(1u << i2c_bus->bus_cfg.scl.pin);
    uint16_t sda_pin = (uint16_t)(1u << i2c_bus->bus_cfg.sda.pin);
    int recovered = 0;

    (void)HAL_I2C_DeInit(&i2c_bus->hi2c);

    gpio_init_output_od(scl_port, scl_pin);
    gpio_init_output_od(sda_port, sda_pin);

    HAL_GPIO_WritePin(scl_port, scl_pin, GPIO_PIN_SET);
    HAL_GPIO_WritePin(sda_port, sda_pin, GPIO_PIN_SET);
    delay_us(I2C_RECOVERY_DELAY_US);

    for (int i = 0; i < I2C_RECOVERY_CLOCKS; ++i)
    {
        if (HAL_GPIO_ReadPin(sda_port, sda_pin) == GPIO_PIN_SET)
        {
            recovered = 1;
            break;
        }

        HAL_GPIO_WritePin(scl_port, scl_pin, GPIO_PIN_RESET);
        delay_us(I2C_RECOVERY_DELAY_US);
        HAL_GPIO_WritePin(scl_port, scl_pin, GPIO_PIN_SET);
        delay_us(I2C_RECOVERY_DELAY_US);

        if (HAL_GPIO_ReadPin(scl_port, scl_pin) == GPIO_PIN_RESET)
        {
            break;
        }
    }

    if (HAL_GPIO_ReadPin(sda_port, sda_pin) == GPIO_PIN_SET)
    {
        recovered = 1;
    }

    if (recovered)
    {
        HAL_GPIO_WritePin(sda_port, sda_pin, GPIO_PIN_RESET);
        delay_us(I2C_RECOVERY_DELAY_US);
        HAL_GPIO_WritePin(scl_port, scl_pin, GPIO_PIN_SET);
        delay_us(I2C_RECOVERY_DELAY_US);
        recovered = HAL_GPIO_ReadPin(scl_port, scl_pin) == GPIO_PIN_SET;
        HAL_GPIO_WritePin(sda_port, sda_pin, GPIO_PIN_SET);
        delay_us(I2C_RECOVERY_DELAY_US);
    }

    gpio_init_af_od(scl_port, scl_pin, i2c_bus->bus_cfg.scl.af);
    gpio_init_af_od(sda_port, sda_pin, i2c_bus->bus_cfg.sda.af);

    if (i2c_configure_hal(i2c_bus) != 0)
    {
        return -PosixError_EIO;
    }

    return recovered ? 0 : -PosixError_ENOTRECOVERABLE;
}

int i2c_write(void *bus, const i2c_device_cfg_t *dev_cfg, struct i2c_transfer *transfer)
{
    if (bus == 0 || dev_cfg == 0 || transfer == 0 || transfer->tx == 0 || dev_cfg->address > 0x7fU ||
        transfer->tx_len <= 0 || transfer->tx_len > 0xffff)
    {
        return -PosixError_EINVAL;
    }

    struct i2c_bus *i2c_bus = (struct i2c_bus *)bus;
    
    HAL_StatusTypeDef res = HAL_I2C_Master_Transmit(
        &i2c_bus->hi2c,
        (uint16_t)(dev_cfg->address << 1),
        (uint8_t *)transfer->tx,
        (uint16_t)transfer->tx_len,
        transfer->timeout);

    return res == HAL_OK ? 0 : i2c_transfer_error(i2c_bus, res);
}

int i2c_read(void *bus, const i2c_device_cfg_t *dev_cfg, struct i2c_transfer *transfer)
{
    if (bus == 0 || dev_cfg == 0 || transfer == 0 || transfer->rx == 0 || dev_cfg->address > 0x7fU ||
        transfer->rx_len <= 0 || transfer->rx_len > 0xffff)
    {
        return -PosixError_EINVAL;
    }

    struct i2c_bus *i2c_bus = (struct i2c_bus *)bus;
    HAL_StatusTypeDef res = HAL_I2C_Master_Receive(
        &i2c_bus->hi2c,
        (uint16_t)(dev_cfg->address << 1),
        transfer->rx,
        (uint16_t)transfer->rx_len,
        transfer->timeout);

    return res == HAL_OK ? 0 : i2c_transfer_error(i2c_bus, res);
}

int i2c_write_read(void *bus, const i2c_device_cfg_t *dev_cfg, const struct i2c_transfer *transfer)
{
    if (bus == 0 || dev_cfg == 0 || transfer == 0 || transfer->tx == 0 || transfer->rx == 0 ||
        dev_cfg->address > 0x7fU || transfer->tx_len <= 0 || transfer->rx_len <= 0 || transfer->tx_len > 0xffff ||
        transfer->rx_len > 0xffff)
    {
        return -PosixError_EINVAL;
    }

    struct i2c_bus *i2c_bus = (struct i2c_bus *)bus;
    if (transfer->tx_len > 2)
    {
        return -PosixError_EINVAL;
    }

    uint16_t mem_address = transfer->tx[0];
    uint16_t mem_address_size = I2C_MEMADD_SIZE_8BIT;
    if (transfer->tx_len == 2)
    {
        mem_address = ((uint16_t)transfer->tx[0] << 8) | transfer->tx[1];
        mem_address_size = I2C_MEMADD_SIZE_16BIT;
    }

    HAL_StatusTypeDef res = HAL_I2C_Mem_Read(
        &i2c_bus->hi2c,
        (uint16_t)(dev_cfg->address << 1),
        mem_address,
        mem_address_size,
        transfer->rx,
        (uint16_t)transfer->rx_len,
        transfer->timeout);

    return res == HAL_OK ? 0 : i2c_transfer_error(i2c_bus, res);
}

int i2c_deinit(void *bus)
{
    if (bus == 0)
    {
        return -PosixError_EINVAL;
    }

    struct i2c_bus *i2c_bus = (struct i2c_bus *)bus;
    if (HAL_I2C_DeInit(&i2c_bus->hi2c) != HAL_OK)
    {
        return -PosixError_EIO;
    }
    
    i2c_disable_clock(i2c_bus->hi2c.Instance);
    i2c_bus->in_use = 0;
    return 0;
}

int i2c_init_device(const i2c_device_cfg_t *dev_cfg)
{
    if (dev_cfg == 0 || dev_cfg->address > 0x7fU)
    {
        return -PosixError_EINVAL;
    }

    if (dev_cfg->enable.port != (uintptr_t)0)
    {
        GPIO_TypeDef *port = (GPIO_TypeDef *)dev_cfg->enable.port;
        uint16_t pin = (uint16_t)(1u << dev_cfg->enable.pin);
        uint8_t active_low = dev_cfg->enable.active_low ? 1u : 0u;

        int active_state = active_low ? GPIO_PIN_RESET : GPIO_PIN_SET;
        gpio_enable_clock(port);
        gpio_init_output(port, pin);
        HAL_GPIO_WritePin(port, pin, active_state);
        /* Let the device power up before the first transaction. */
        delay_us(I2C_ENABLE_SETTLE_US);
        return 0;
    }

    return 0;
}

int i2c_deinit_device(const i2c_device_cfg_t *dev_cfg)
{
    if (dev_cfg == 0 || dev_cfg->address > 0x7fU)
    {
        return -PosixError_EINVAL;
    }

    if (dev_cfg->enable.port != (uintptr_t)0)
    {
        GPIO_TypeDef *port = (GPIO_TypeDef *)dev_cfg->enable.port;
        uint16_t pin = (uint16_t)(1u << dev_cfg->enable.pin);
        uint8_t active_low = dev_cfg->enable.active_low ? 1u : 0u;

        int inactive_state = active_low ? GPIO_PIN_SET : GPIO_PIN_RESET;
        HAL_GPIO_WritePin(port, pin, inactive_state);
        return 0;
    }

    return 0;
}
