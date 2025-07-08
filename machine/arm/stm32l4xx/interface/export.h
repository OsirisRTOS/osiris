#pragma once

// lib.c
void init_hal(void);

// uart.c
int init_debug_uart(void);
int write_debug_uart(const char *buf, int len);

// sched.c
void reschedule(void);

//instru.c
void dwt_init(void);
void dwt_reset(void);
long dwt_read(void);
float dwt_read_ns(void);
float dwt_cycles_to_ns(long cycles);
