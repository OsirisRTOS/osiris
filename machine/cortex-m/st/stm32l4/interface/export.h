#pragma once

// lib.c
unsigned long long systick_freq(void);
int init_hal(void);

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

// clock.c
void SystemClock_Config(void);

unsigned long long monotonic_now(void);
unsigned long long monotonic_freq(void);
unsigned long long rtc_raw(void);
int set_rtc_raw(unsigned long long time);

unsigned long rtc_backup_register(unsigned char index);
void set_rtc_backup_register(unsigned char index, unsigned long value);
