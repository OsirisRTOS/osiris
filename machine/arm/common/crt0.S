.syntax unified
.thumb

.extern __data_start
.extern __data
.extern __data_end
.extern __bss_start
.extern __bss_end

.thumb_func
.global bootstrap
bootstrap:
    @ Copy initialized data from flash to RAM.
    ldr r0, =__data
    ldr r1, =__data_start
    ldr r2, =__data_end
1:  cmp r1, r2
    itt lt
    ldrlt r3, [r0], #4
    strlt r3, [r1], #4
    blt 1b
    @ Zero out the BSS section.
    ldr r1, =__bss_start
    ldr r2, =__bss_end
    eor r3, r3, r3
2:  cmp r1, r2
    it lt
    strlt r3, [r1], #4
    blt 2b
    @ Call the pre_init function.
    bl pre_init
    @ If main returns, loop forever.
hang:
    b hang