.syntax unified
.cpu cortex-m4
.thumb

.global vector_table

.type vector_table, %object
.section .ivt.core, "a", %progbits
vector_table:
.word __stack_top
.word reset_hndlr
.word nmi_hndlr
.word hard_fault_hndlr
.word mem_manage_fault_hndlr
.word bus_fault_hndlr
.word usage_fault_hndlr
.word 0
.word 0
.word 0
.word 0
.word svc_hndlr
.word debug_mon_hndlr
.word 0
.word pendsv_hndlr
.word systick_hndlr

.macro hndl_weak_endl fnct
    .thumb_func
    .weak \fnct
    .type \fnct, %function
    
    \fnct:
        0: b 0b
        .size \fnct, . - \fnct
.endm

.text
.align

hndl_weak_endl nmi_hndlr
hndl_weak_endl debug_mon_hndlr

.thumb_func
.global reset_hndlr
reset_hndlr:
    ldr r1,=__stack_top
    mov sp, r1
    bl _main


