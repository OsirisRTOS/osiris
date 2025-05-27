#pragma once

// lib.c
void init_hal(void);

// uart.c
int init_debug_uart(void);
int write_debug_uart(const char *buf, int len);

// sched.c
void reschedule(void);
