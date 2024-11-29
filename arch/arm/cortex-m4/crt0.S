.syntax unified
.cpu cortex-m4
.thumb

.global vector_table

.type vector_table, %object
.section .ivt, "a", %progbits
vector_table:
.word __stack
.word reset_hndlr
.word nmi_hndlr
.word hard_fault_hndlr
.word mem_mngmt_hndlr
.word bus_fault_hndlr
.word usage_fault_hndlr
.word 0
.word 0
.word 0
.word 0
.word svc_hndlr
.word debug_mon_hndlr
.word 0
.word pend_sv_hndlr
.word sys_tick_hndlr

.macro hndl_weak_endl fnct
    .thumb_func
    .weak \fnct
    .type \fnct, %function
    
    \fnct:
        0: b 0b
        .size \fnct, . - \fnct
.endm

.include "isr_ext.S"

.text
.align

hndl_weak_endl nmi_hndlr
hndl_weak_endl hard_fault_hndlr
hndl_weak_endl mem_mngmt_hndlr
hndl_weak_endl bus_fault_hndlr
hndl_weak_endl usage_fault_hndlr
hndl_weak_endl svc_hndlr
hndl_weak_endl debug_mon_hndlr
hndl_weak_endl pend_sv_hndlr
hndl_weak_endl sys_tick_hndlr

.thumb_func
.global reset_hndlr
reset_hndlr:
    ldr r1,=__stack_end
    mov sp, r1
    bl _main


