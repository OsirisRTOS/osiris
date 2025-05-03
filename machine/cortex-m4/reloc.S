
// This assembly startup script performs a relocation of the GOT entries with an offset specified at the stack.

.syntax unified
.cpu cortex-m4
.thumb

.thumb_func
.global relocate_got
relocate_got:
    // R0 = __got_start (RAM)
    // R1 = __got_end   (RAM) / tmp
    // R2 = offset (from bootloader) / tmp
    // R3 = relocated flash got start
    ldr r0, =__got_start

    // Load the address of the GOT in flash.
    ldr r3, =__got 

    // Apply the offset to the GOT start address in flash.
    ldr r2, [sp]
    add r3, r3, r2

    b relocate_got_loop_check

relocate_got_loop_inc:
    // Increment the GOT entry pointer.
    add r0, r0, #4
    // Increment the GOT entry pointer in flash.
    add r3, r3, #4

relocate_got_loop_check:
    ldr r1, =__got_end
    // Check if we have reached the end of the GOT.
    cmp r0, r1
    beq relocate_got_done

    // Load the got entry.
    ldr r2, [r3]

    // Check if the entry is between the flash start and end.
    ldr r1, =__flash_start
    cmp r2, r1
    blt relocate_got_loop_store

    ldr r1, =__flash_end
    cmp r2, r1
    bgt relocate_got_loop_store

    // Load the offset that should be added to each GOT entry in RAM.
    ldr r1, [sp]
    // If the entry is between the flash start and end, we need to add the offset.
    add r2, r2, r1

relocate_got_loop_store:
    // Store the entry which is stored in r2 at the actual GOT location in RAM.
    str r2, [r0]

    b relocate_got_loop_inc

relocate_got_done:
    dsb
    bx lr
