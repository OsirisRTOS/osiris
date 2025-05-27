
#define STM32L4R5xx
#include <stm32l4xx_hal.h>

void reschedule(void)
{
    SCB->ICSR |= SCB_ICSR_PENDSVSET_Msk; // Trigger PendSV exception
    __ISB();
    __DSB();
}