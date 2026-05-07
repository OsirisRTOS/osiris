#include "lib.h"
#include "export.h"
#include "gpio.h"
#include "stm32l4xx.h"
#include "stm32l4xx_hal_gpio.h"
#include "stm32l4xx_hal_rcc.h"
#include "stm32l4xx_ll_spi.h"

#include <stm32l4xx_hal.h>

#define SPI_SLOT_COUNT 4

struct spi_bus {
    uint8_t in_use;
    SPI_HandleTypeDef hspi;
};

static struct spi_bus spi_buses[SPI_SLOT_COUNT];

static void spi_enable_clock(SPI_TypeDef *instance)
{
#if defined(SPI1)
    if (instance == SPI1)
    {
        __HAL_RCC_SPI1_CLK_ENABLE();
        return;
    }
#endif
#if defined(SPI2)
    if (instance == SPI2)
    {
        __HAL_RCC_SPI2_CLK_ENABLE();
        return;
    }
#endif
#if defined(SPI3)
    if (instance == SPI3)
    {
        __HAL_RCC_SPI3_CLK_ENABLE();
        return;
    }
#endif
#if defined(SPI4)
    if (instance == SPI4)
    {
        __HAL_RCC_SPI4_CLK_ENABLE();
        return;
    }
#endif
}

static void spi_disable_clock(SPI_TypeDef *instance)
{
#if defined(SPI1)
    if (instance == SPI1)
    {
        __HAL_RCC_SPI1_CLK_DISABLE();
        return;
    }
#endif
#if defined(SPI2)
    if (instance == SPI2)
    {
        __HAL_RCC_SPI2_CLK_DISABLE();
        return;
    }
#endif
#if defined(SPI3)
    if (instance == SPI3)
    {
        __HAL_RCC_SPI3_CLK_DISABLE();
        return;
    }
#endif
#if defined(SPI4)
    if (instance == SPI4)
    {
        __HAL_RCC_SPI4_CLK_DISABLE();
        return;
    }
#endif
}

static uint32_t spi_bus_clock_hz(SPI_TypeDef *instance)
{
#if defined(SPI1)
    if (instance == SPI1)
    {
        return HAL_RCC_GetPCLK2Freq();
    }
#endif
#if defined(SPI4)
    if (instance == SPI4)
    {
        return HAL_RCC_GetPCLK2Freq();
    }
#endif
    return HAL_RCC_GetPCLK1Freq();
}

static uint32_t spi_prescaler_for_max_hz(uint32_t pclk_hz, uint32_t max_hz)
{
    const uint32_t dividers[] = {2U, 4U, 8U, 16U, 32U, 64U, 128U, 256U};
    const uint32_t prescalers[] = {
        SPI_BAUDRATEPRESCALER_2,
        SPI_BAUDRATEPRESCALER_4,
        SPI_BAUDRATEPRESCALER_8,
        SPI_BAUDRATEPRESCALER_16,
        SPI_BAUDRATEPRESCALER_32,
        SPI_BAUDRATEPRESCALER_64,
        SPI_BAUDRATEPRESCALER_128,
        SPI_BAUDRATEPRESCALER_256,
    };

    for (int i = 0; i < 8; ++i)
    {
        if ((pclk_hz / dividers[i]) <= max_hz)
        {
            return prescalers[i];
        }
    }

    return SPI_BAUDRATEPRESCALER_256;
}

static uint32_t spi_datasize_from_bits(uint8_t bits)
{
    return SPI_DATASIZE_4BIT + (bits - 4U) * 0x100U;
}

void *spi_init(const spi_bus_cfg_t *bus_cfg)
{
    if (bus_cfg == 0)
    {
        return 0;
    }

    struct spi_bus *slot = 0;

    for (int i = 0; i < SPI_SLOT_COUNT; ++i)
    {
        if (!spi_buses[i].in_use)
        {
            slot = &spi_buses[i];
            break;
        }
    }

    if (slot == 0)
    {
        return 0;
    }

    GPIO_TypeDef *sck_port = (GPIO_TypeDef *)bus_cfg->sck.port;
    GPIO_TypeDef *miso_port = (GPIO_TypeDef *)bus_cfg->miso.port;
    GPIO_TypeDef *mosi_port = (GPIO_TypeDef *)bus_cfg->mosi.port;
    uint16_t sck_pin = (uint16_t)(1u << bus_cfg->sck.pin);
    uint16_t miso_pin = (uint16_t)(1u << bus_cfg->miso.pin);
    uint16_t mosi_pin = (uint16_t)(1u << bus_cfg->mosi.pin);

    spi_enable_clock((SPI_TypeDef *)bus_cfg->instance);

    gpio_enable_clock(sck_port);
    gpio_enable_clock(miso_port);
    gpio_enable_clock(mosi_port);

    gpio_init_af(sck_port, sck_pin, bus_cfg->sck.af);
    gpio_init_af(miso_port, miso_pin, bus_cfg->miso.af);
    gpio_init_af(mosi_port, mosi_pin, bus_cfg->mosi.af);

    slot->hspi.Instance = (SPI_TypeDef *)bus_cfg->instance;
    slot->hspi.State = HAL_SPI_STATE_RESET;
    slot->hspi.Init.Mode = SPI_MODE_MASTER;
    // Only support for full-duplex for now.
    slot->hspi.Init.Direction = SPI_DIRECTION_2LINES;
    slot->hspi.Init.NSS = SPI_NSS_SOFT;

    if (HAL_SPI_Init(&slot->hspi) != HAL_OK)
    {
        return 0;
    }

    slot->in_use = 1;
    return (void *)slot;
}

static int spi_select_device(void *bus, const spi_device_cfg_t *dev_cfg)
{
    if (bus == 0 || dev_cfg == 0)
    {
        return -1;
    }

    uint32_t prescaler = spi_prescaler_for_max_hz(
        spi_bus_clock_hz((SPI_TypeDef *)dev_cfg->instance), dev_cfg->max_hz);
    uint32_t polarity = dev_cfg->cpol ? SPI_POLARITY_HIGH : SPI_POLARITY_LOW;
    uint32_t phase = dev_cfg->cpha ? SPI_PHASE_2EDGE : SPI_PHASE_1EDGE;
    uint32_t datasize = spi_datasize_from_bits(dev_cfg->bits_per_word);
    uint32_t bit_order = dev_cfg->bit_order ? SPI_FIRSTBIT_LSB : SPI_FIRSTBIT_MSB;

    SPI_HandleTypeDef *hspi = &((struct spi_bus*)bus)->hspi;

    if (prescaler != hspi->Init.BaudRatePrescaler ||
        polarity != hspi->Init.CLKPolarity ||
        phase != hspi->Init.CLKPhase ||
        datasize != hspi->Init.DataSize ||
        bit_order != hspi->Init.FirstBit)
    {

        while(__HAL_SPI_GET_FLAG(hspi, SPI_FLAG_BSY)) {}
        __HAL_SPI_DISABLE(hspi);

        LL_SPI_SetBaudRatePrescaler(hspi->Instance, prescaler);
        LL_SPI_SetClockPolarity(hspi->Instance, polarity);
        LL_SPI_SetClockPhase(hspi->Instance, phase);
        LL_SPI_SetDataWidth(hspi->Instance, datasize);
        LL_SPI_SetTransferBitOrder(hspi->Instance, bit_order);

        __HAL_SPI_ENABLE(hspi);

        hspi->Init.BaudRatePrescaler = prescaler;
        hspi->Init.CLKPolarity = polarity;
        hspi->Init.CLKPhase = phase;
        hspi->Init.DataSize = datasize;
        hspi->Init.FirstBit = bit_order;
    }

    GPIO_TypeDef *port = (GPIO_TypeDef *)dev_cfg->cs.port;
    uint16_t pin = (uint16_t)(1u << dev_cfg->cs.pin);
    uint32_t active_state = dev_cfg->cs.active_low ? GPIO_PIN_RESET : GPIO_PIN_SET;
    HAL_GPIO_WritePin(port, pin, active_state);

    delay_us(dev_cfg->cs_setup_delay_us);
    return 0;
}

int spi_transfer(void *bus, const spi_device_cfg_t *dev_cfg, const struct spi_transfer *transfer)
{
    if (bus == 0 || dev_cfg == 0 || transfer == 0)
    {
        return -1;
    }

    if (transfer->word_count > 0xffff)
    {
        return -1;
    }

    if (spi_select_device(bus, dev_cfg) != 0)
    {
        return -1;
    }

    struct spi_bus *spi_bus = (struct spi_bus*)bus;
    uint32_t hz = spi_bus_clock_hz((SPI_TypeDef *)dev_cfg->instance);
    uint32_t time_seconds = (transfer->word_count * dev_cfg->bits_per_word) / hz;
    // time_seconds * 1000 + margin
    uint32_t timeout_ms = time_seconds * 1000U + 100U;

    HAL_StatusTypeDef res = HAL_SPI_TransmitReceive(
        &spi_bus->hspi,
        (uint8_t *)transfer->tx_words,
        (uint8_t *)transfer->rx_words,
        (uint16_t)transfer->word_count,
        timeout_ms);

    delay_us(dev_cfg->cs_hold_delay_us);

    GPIO_TypeDef *port = (GPIO_TypeDef *)dev_cfg->cs.port;
    uint16_t pin = (uint16_t)(1u << dev_cfg->cs.pin);
    uint32_t inactive_state = dev_cfg->cs.active_low ? GPIO_PIN_SET : GPIO_PIN_RESET;
    HAL_GPIO_WritePin(port, pin, inactive_state);

    delay_us(dev_cfg->cs_inactive_delay_us);

    if (res != HAL_OK)
    {
        return -1;
    }
    return 0;
}

int spi_deinit(void *bus)
{
    struct spi_bus *spi_bus = (struct spi_bus*)bus;
    if (HAL_SPI_DeInit(&spi_bus->hspi) != HAL_OK)
    {
        return -1;
    }

    spi_disable_clock(spi_bus->hspi.Instance);
    spi_bus->in_use = 0;
    return 0;
}

int spi_init_device(const spi_device_cfg_t *dev_cfg)
{
    if (dev_cfg->enable.port != (uintptr_t)0)
    {
        GPIO_TypeDef *port = (GPIO_TypeDef *)dev_cfg->enable.port;
        uint16_t pin = (uint16_t)(1u << dev_cfg->enable.pin);
        uint8_t active_low = dev_cfg->enable.active_low ? 1u : 0u;

        int active_state = active_low ? GPIO_PIN_RESET : GPIO_PIN_SET;
        gpio_enable_clock(port);
        HAL_GPIO_WritePin(port, pin, active_state);
        gpio_init_output(port, pin);
    }

    GPIO_TypeDef *port = (GPIO_TypeDef *)dev_cfg->cs.port;
    uint16_t pin = (uint16_t)(1u << dev_cfg->cs.pin);
    uint32_t inactive_state = dev_cfg->cs.active_low ? GPIO_PIN_SET : GPIO_PIN_RESET;
    gpio_enable_clock(port);
    HAL_GPIO_WritePin(port, pin, inactive_state);
    gpio_init_output(port, pin);
    return 0;
}

int spi_deinit_device(const spi_device_cfg_t *dev_cfg)
{
    if (dev_cfg == 0 || dev_cfg->enable.port == (uintptr_t)0)
    {
        return 0;
    }

    GPIO_TypeDef *port = (GPIO_TypeDef *)dev_cfg->enable.port;
    uint16_t pin = (uint16_t)(1u << dev_cfg->enable.pin);
    uint8_t active_low = dev_cfg->enable.active_low ? 1u : 0u;

    int inactive_state = active_low ? GPIO_PIN_SET : GPIO_PIN_RESET;
    HAL_GPIO_WritePin(port, pin, inactive_state);
    return 0;
}
