#pragma once

#include <stdint.h>

/* Sentinel returned on init/conversion failure (see export.h). */
#define HAL_MCU_TEMP_ERROR INT32_MIN

/* Read the STM32L4 internal junction temperature sensor.
 * Returns the die temperature in milli-degrees Celsius, or
 * HAL_MCU_TEMP_ERROR on failure. */
int32_t hal_mcu_temp_millidegc(void);
