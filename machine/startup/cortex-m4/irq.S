.syntax unified
.thumb

.text
.align

# References used to write this code:
#
# * Calling conventions for Thumb functions: https://developer.arm.com/documentation/dui0041/c/Thumb-Procedure-Call-Standard?lang=en
# * Programming manual: https://www.st.com/resource/en/programming_manual/pm0214-stm32-cortexm4-mcus-and-mpus-programming-manual-stmicroelectronics.pdf
#
# -----------------------------------------------------------------------------------------------

.thumb_func
.global pendsv_hndlr
pendsv_hndlr:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, msp
    mrsne r0, psp

    @ Check if we need to save the floating point registers.
    tst lr, #0x10
    it eq
    vstmdbeq r0!, {s16-s31}
    
    stmdb r0!, {r4-r11, lr}

    @ This function takes a pointer to the thread's context as the first argument.
    blx sched_enter

    ldmia r0!, {r4-r11, lr}

    @ Check if we need to restore the floating point registers.
    tst lr, #0x10
    it eq
    vldmiaeq r0!, {s16-s31}

    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    msreq msp, r0
    msrne psp, r0

    @ See Cortex-M4 errata.
    isb

    @ Return to the thread
    bx lr
    .align 2
    .ltorg

.thumb_func
.global irq_enter_no_switch_fp
irq_enter_no_switch_fp:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, msp
    mrsne r0, psp

    @ Check if we need to save the floating point registers.
    @tst lr, #10
    @ite eq
    @vstmdbeq r0!, {s16-s31}
    
    @ Save the EXEC_RETURN of this thread.
    stmdb r0, {r4-r11, lr}

    @ This function takes a pointer to the thread's context as the first argument.
    blx sched_enter

    @ Restore the EXEC_RETURN of the new thread.
    ldmia r0!, {r4-r11, lr}

    @ Check if we need to restore the floating point registers.
    @tst lr, #10
    @ite eq
    @vldmiaeq r0!, {s16-s31}

    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    msreq msp, r0
    msrne psp, r0

    @ Return to the thread
    bx lr

.thumb_func
.global svc_hndlr
svc_hndlr:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, msp
    mrsne r0, psp

    @ We don't need to save the other general purpose registers, because the syscall obeys the AAPCS calling convention.
    b _syscall_hndlr


.thumb_func
.global hard_fault_hndlr
hard_fault_hndlr:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, msp
    mrsne r0, psp

    @ Save the frame pointer as the second argument.
    mov r1, r11

    b handle_hard_fault

.thumb_func
.global mem_manage_fault_hndlr
mem_manage_fault_hndlr:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, msp
    mrsne r0, psp

    @ Save the frame pointer as the second argument.
    mov r1, r11

    b handle_mem_manage_fault

.thumb_func
.global bus_fault_hndlr
bus_fault_hndlr:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, msp
    mrsne r0, psp

    @ Save the frame pointer as the second argument.
    mov r1, r11

    b handle_bus_fault

.thumb_func
.global usage_fault_hndlr
usage_fault_hndlr:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, msp
    mrsne r0, psp

    @ Save the frame pointer as the second argument.
    mov r1, r11

    b handle_usage_fault
    