.syntax unified
.cpu cortex-m4
.thumb

.text
.align

# References used to write this code:
#
# * Calling conventions for Thumb functions: https://developer.arm.com/documentation/dui0041/c/Thumb-Procedure-Call-Standard?lang=en
# * Programming manual: https://www.st.com/resource/en/programming_manual/pm0214-stm32-cortexm4-mcus-and-mpus-programming-manual-stmicroelectronics.pdf
#
# -----------------------------------------------------------------------------------------------

# .thumb_func
# .global irq_enter_switch
# irq_enter_switch:
#     @ Check which stack we need to use.
#     tst lr, 0x7
#     ite eq
#     mrseq r0, psp
#     mrsne r0, msp
# 
#     @ Check if we need to save the floating point registers.
#     tst lr, 0x10
#     ite eq
#     vstmdbeq r0!, {s16-s31}
# 
#     @ Save the general purpose registers.
#     stmdb r0!, {r4-r11}
# 
#     @ This function has to return a uint32_t which represents EXC_RETURN.
#     mrs r0, lr
#     bl sched_call
#     msr lr, r0
# 
#     @ Check which stack we need to use.
#     tst lr, 0x7
#     it eq
#     mrseq r0, psp
#     mrsne r0, msp
# 
#     @ Restore the general purpose registers.
#     ldmia r0!, {r4-r11}
# 
#     @ Check if we need to restore the floating point registers.
#     tst lr, 0x10
#     it eq
#     vldmiaeq r0!, {s16-s31}
# 
#     @ Return to the task
#     bx lr

.thumb_func
.global irq_enter_switch_fp
irq_enter_switch_fp:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, psp
    mrsne r0, msp

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
    msreq psp, r0
    msrne msp, r0

    @ See Cortex-M4 errata.
    isb

    @ Return to the thread
    bx lr

.thumb_func
.global irq_enter_no_switch_fp
irq_enter_no_switch_fp:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, psp
    mrsne r0, msp

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
    msreq psp, r0
    msrne msp, r0

    @ Return to the thread
    bx lr

.thumb_func
.global syscall_enter
syscall_enter:
    @ Check which stack we need to use.
    tst lr, #4
    ite eq
    mrseq r0, psp
    mrsne r0, msp

    @ We don't need to save the other general purpose registers, because the syscall obeys the AAPCS calling convention.
    b _syscall_hndlr
