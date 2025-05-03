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

.text
.align

hndl_weak_endl nmi_hndlr
hndl_weak_endl hard_fault_hndlr
hndl_weak_endl mem_mngmt_hndlr
hndl_weak_endl bus_fault_hndlr
hndl_weak_endl usage_fault_hndlr
hndl_weak_endl debug_mon_hndlr

.thumb_func
.global reset_hndlr
reset_hndlr:
    // Check if sp is equal to __stack_top - 4. If yes, we got an offset from the bootloader.
    // If not, we are booting without a bootloader.
    ldr r0, =__stack_top
    sub r0, r0, #4
    cmp sp, r0
    bne no_bootloader

    //bl relocate_got

    ldr r0, [sp]
    ldr r9, =__data_start
    bl _main

    b loop

no_bootloader:
    // If we are here, we are booting without a bootloader.
    // We need to set the stack pointer to the top of the stack.
    ldr r0,=__stack_top
    mov sp, r0

    // Call the main function.
    mov r0, #0
    bl _main

loop:
    b loop


