#pragma once

#include <stdint.h>

void init_clock_cfg(void);
unsigned long long monotonic_now(void);
unsigned long long monotonic_freq(void);
void delay_us(uint32_t delay_us);
