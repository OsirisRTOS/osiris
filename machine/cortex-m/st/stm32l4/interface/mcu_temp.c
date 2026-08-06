/*
 * STM32L4 internal junction-temperature-sensor reading.
 *
 * Reads ADC1 channel 17 (ADC_CHANNEL_TEMPSENSOR), the on-die temperature
 * sensor, and converts the raw code to a temperature using the factory
 * calibration values.
 *
 * Reference: RM0432 (STM32L4+ reference manual) section 21.4.32
 * "Temperature sensor". The sensor needs a long ADC sampling time
 * (640.5 cycles here) for its high output impedance to settle.
 *
 * Conversion formula (applied by __HAL_ADC_CALC_TEMPERATURE):
 *
 *   T = (TS_CAL2_TEMP - TS_CAL1_TEMP)
 *       --------------------------------- * (TS_DATA - TS_CAL1) + TS_CAL1_TEMP
 *       (TS_CAL2       - TS_CAL1)
 *
 * where TS_CAL1/TS_CAL2 are the factory-programmed ADC codes at the two
 * calibration temperatures, read from CMSIS-defined addresses. The macro
 * also rescales TS_DATA for VREF+ (calibration was performed at 3.0 V) and
 * for ADC resolution. The typical-slope (Avg_Slope / V25) formula is NOT
 * used: it is inaccurate on STM32L4.
 */

#include "mcu_temp.h"

#include "stm32l4xx_hal.h"
#include "stm32l4xx_hal_adc.h"
#include "stm32l4xx_hal_adc_ex.h"

/* Nucleo VREF+ in millivolts; the temperature macro scales for it. */
#define MCU_TEMP_VDDA_MV 3300U

/* Samples averaged per read to reduce noise. */
#define MCU_TEMP_SAMPLES 16U

/* Per-conversion poll timeout, milliseconds. */
#define MCU_TEMP_CONV_TIMEOUT_MS 10U

int32_t hal_mcu_temp_millidegc(void)
{
  ADC_HandleTypeDef hadc = {0};
  ADC_ChannelConfTypeDef chan = {0};

  __HAL_RCC_ADC_CLK_ENABLE();

  /* ADC1, single conversion, software trigger, 12-bit right-aligned.
   * Synchronous clock from PCLK2 divided by 4: 80 MHz / 4 = 20 MHz,
   * within the STM32L4 datasheet ADC clock maximum. */
  hadc.Instance = ADC1;
  hadc.Init.ClockPrescaler = ADC_CLOCK_SYNC_PCLK_DIV4;
  hadc.Init.Resolution = ADC_RESOLUTION_12B;
  hadc.Init.DataAlign = ADC_DATAALIGN_RIGHT;
  hadc.Init.ScanConvMode = ADC_SCAN_DISABLE;
  hadc.Init.EOCSelection = ADC_EOC_SINGLE_CONV;
  hadc.Init.LowPowerAutoWait = DISABLE;
  hadc.Init.ContinuousConvMode = DISABLE;
  hadc.Init.NbrOfConversion = 1;
  hadc.Init.DiscontinuousConvMode = DISABLE;
  hadc.Init.ExternalTrigConv = ADC_SOFTWARE_START;
  hadc.Init.ExternalTrigConvEdge = ADC_EXTERNALTRIGCONVEDGE_NONE;
  hadc.Init.DMAContinuousRequests = DISABLE;
  hadc.Init.Overrun = ADC_OVR_DATA_OVERWRITTEN;
  hadc.Init.OversamplingMode = DISABLE;

  if (HAL_ADC_Init(&hadc) != HAL_OK)
  {
    return HAL_MCU_TEMP_ERROR;
  }

  /* Internal temperature sensor on regular rank 1. The longest sample
   * time available is required for the sensor's settling (RM0432
   * section 21.4.32). */
  chan.Channel = ADC_CHANNEL_TEMPSENSOR;
  chan.Rank = ADC_REGULAR_RANK_1;
  chan.SamplingTime = ADC_SAMPLETIME_640CYCLES_5;
  chan.SingleDiff = ADC_SINGLE_ENDED;
  chan.OffsetNumber = ADC_OFFSET_NONE;
  chan.Offset = 0;

  if (HAL_ADC_ConfigChannel(&hadc, &chan) != HAL_OK)
  {
    HAL_ADC_DeInit(&hadc);
    return HAL_MCU_TEMP_ERROR;
  }

  /* Calibrate the ADC once before the first conversion. */
  if (HAL_ADCEx_Calibration_Start(&hadc, ADC_SINGLE_ENDED) != HAL_OK)
  {
    HAL_ADC_DeInit(&hadc);
    return HAL_MCU_TEMP_ERROR;
  }

  /* Average MCU_TEMP_SAMPLES conversions. */
  uint32_t accum = 0;
  for (uint32_t i = 0; i < MCU_TEMP_SAMPLES; i++)
  {
    if (HAL_ADC_Start(&hadc) != HAL_OK)
    {
      HAL_ADC_DeInit(&hadc);
      return HAL_MCU_TEMP_ERROR;
    }
    if (HAL_ADC_PollForConversion(&hadc, MCU_TEMP_CONV_TIMEOUT_MS) != HAL_OK)
    {
      HAL_ADC_Stop(&hadc);
      HAL_ADC_DeInit(&hadc);
      return HAL_MCU_TEMP_ERROR;
    }
    accum += HAL_ADC_GetValue(&hadc);
    HAL_ADC_Stop(&hadc);
  }

  HAL_ADC_DeInit(&hadc);

  uint32_t adc_data = accum / MCU_TEMP_SAMPLES;

  /* Guard the divide-by-zero in the conversion (TS_CAL2 == TS_CAL1).
   * This cannot happen on real silicon but keeps the shim defensive. */
  if ((int32_t)*TEMPSENSOR_CAL2_ADDR == (int32_t)*TEMPSENSOR_CAL1_ADDR)
  {
    return HAL_MCU_TEMP_ERROR;
  }

  /* __HAL_ADC_CALC_TEMPERATURE yields whole degrees Celsius; scale to
   * milli-degrees for the integer interface. */
  int32_t degc = __HAL_ADC_CALC_TEMPERATURE(MCU_TEMP_VDDA_MV, adc_data,
                                            ADC_RESOLUTION_12B);

  return degc * 1000;
}
