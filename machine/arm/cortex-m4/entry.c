extern uint32_t __bss_start;
extern uint32_t __bss_end;

extern uint32_t __data_start;
extern uint32_t __data;
extern uint32_t __data_end;

typedef void(*func_t)(void);

extern func_t __init_array_start;
extern func_t __init_array_end;
extern func_t __fini_array_start;
extern func_t __fini_array_end;
