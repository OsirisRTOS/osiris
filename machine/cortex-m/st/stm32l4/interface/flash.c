#include "export.h"
#include "lib.h"
#include "stm32l4xx_hal.c"
#include "stm32l4xx.h"
#include "stm32l4xx_hal_cortex.h"
#include "stm32l4xx_hal_def.h"
#include "stm32l4xx_hal_flash.h"
#include "stm32l4xx_hal_flash_ex.h"
#include "stm32l4xx_hal_gpio.h"
#include "stm32l4xx_hal_rcc.h"
#include "stm32l4xx_hal_rcc_ex.h"
#include "stm32l4xx_ll_utils.h"
#include <assert.h>
#include <stdbool.h>
#include <stdint.h>

#define ONE_MEGABYTE (1024 * 1024)

// Whether the flash busy bit is set - true if busy, false otherwise
bool flash_is_busy(void) { return __HAL_FLASH_GET_FLAG(FLASH_FLAG_BSY) != 0U; }

int flash_size(void) {
  uint32_t flash_size_kb = LL_GetFlashSize();
  return flash_size_kb * 1024;
}

// Check if dual-bank mode is enabled
bool flash_is_dual_bank(void) {
  // Manual: "For 1-Mbyte and 512-Kbyte Flash memory devices, do not care about
  // DBANK"
  uint32_t optr_mask = FLASH_OPTR_DB1M;
#if defined(FLASH_OPTR_DUALBANK)
  // L47x..L4A6
  if (flash_size() > ONE_MEGABYTE) {
    optr_mask = FLASH_OPTR_DUALBANK;
  }
#elif defined(FLASH_OPTR_DBANK)
  // L4+
  if (flash_size() > ONE_MEGABYTE) {
    optr_mask = FLASH_OPTR_DBANK;
  }
#endif
  // For devices <1MB, just read DB1M
  return (READ_BIT(FLASH->OPTR, optr_mask) != 0U);
}

int flash_page_size() {
  if (flash_is_dual_bank()) {
    return 4 * 1024; // 4KB page size in dual-bank mode
  } else {
    return 8 * 1024; // 8KB page size in single-bank mode
  }
}

int flash_page_count() { return flash_size() / flash_page_size(); }

// Block until the most recently issued flash operation finishes, or until
// timeout_ms milliseconds have elapsed (measured against HAL_GetTick).
// Returns the underlying HAL status (HAL_OK / HAL_TIMEOUT / HAL_ERROR).
uint32_t flash_wait_for_last_operation(uint32_t timeout_ms) {
  return FLASH_WaitForLastOperation(timeout_ms);
}

// Unlock the flash for writing.
uint32_t flash_unlock(void) {
  if (flash_is_busy()) {
    return ERR_FLASH_BUSY;
  }
  // Check if the flash is already unlocked.
  // Note that the HAL unlock function also checks this,
  // but we want to prevent usage of the flash by multiple call sites at the
  // same time, so we prefer to have one of them fail.
  if (READ_BIT(FLASH->CR, FLASH_CR_LOCK) == 0U) {
    return ERR_FLASH_DOUBLE_UNLOCK;
  }

  HAL_StatusTypeDef status = HAL_FLASH_Unlock();
  if (status != HAL_OK) {
    return ERR_FLASH_NOT_UNLOCKED;
  }

  return HAL_OK;
}

// Note: must wait for flash to not be busy before calling this function.
uint32_t flash_lock(void) {
  if (flash_is_busy()) {
    return ERR_FLASH_BUSY;
  }
  // Note: always returns HAL_OK.
  return HAL_FLASH_Lock();
}

// Erase the flash page identified by `page_index` (a page number, not an
// address). The bank is derived automatically from the index.
uint32_t flash_erase(uint32_t page_index, uint32_t timeout_ms) {
  if (flash_is_busy()) {
    return ERR_FLASH_BUSY;
  }
  if (page_index >= flash_page_count()) {
    return ERR_FLASH_INVALID_PAGE;
  }

  // Check which bank we are on
  uint32_t bank = FLASH_BANK_1;
#ifdef FLASH_BANK_2
  const uint32_t half_page_count = flash_page_count() / 2;
  if (flash_is_dual_bank() && page_index >= half_page_count) {
    bank = FLASH_BANK_2;
    page_index -= half_page_count;
  }
#endif

  FLASH_PageErase(page_index, bank);

  HAL_StatusTypeDef status = FLASH_WaitForLastOperation(timeout_ms);
  if (status != HAL_OK) {
    return HAL_FLASH_GetError();
  }

  return HAL_OK;
}

uint32_t flash_program(uint32_t start_address, const uint64_t *data,
                       uint32_t length, uint32_t timeout_ms) {
  uint32_t total_bytes = length * (uint32_t)sizeof(uint64_t);
  uint32_t end_address = start_address + total_bytes;
  if (start_address < FLASH_BASE || end_address < start_address ||
      end_address > FLASH_BASE + (uint32_t)flash_size()) {
    return ERR_FLASH_ILLEGAL;
  }
  if ((start_address & 0x7U) != 0U) {
    return ERR_FLASH_ILLEGAL;
  }

  if (flash_is_busy()) {
    return ERR_FLASH_BUSY;
  }

  uint32_t tickstart = HAL_GetTick();

  for (uint32_t dw_written = 0; dw_written < length; dw_written++) {
    uint32_t elapsed = HAL_GetTick() - tickstart;
    if (elapsed >= timeout_ms) {
      return ERR_FLASH_TIMEOUT;
    }
    // disable interrupts, as flash_program must write two 32-bit words without
    // interruptions. Later restore exact state

    uint32_t primask = __get_PRIMASK();
    __disable_irq();
    HAL_StatusTypeDef status = HAL_FLASH_Program(
        FLASH_TYPEPROGRAM_DOUBLEWORD,
        start_address + (dw_written * sizeof(uint64_t)), data[dw_written]);
    __set_PRIMASK(primask);

    if (status != HAL_OK) {
      return HAL_FLASH_GetError();
    }

    elapsed = HAL_GetTick() - tickstart;
    if (elapsed >= timeout_ms) {
      return ERR_FLASH_TIMEOUT;
    }
    status = FLASH_WaitForLastOperation(timeout_ms - elapsed);
    if (status != HAL_OK) {
      return HAL_FLASH_GetError();
    }
  }

  return FLASH_OK;
}
