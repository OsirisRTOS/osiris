qemu-system-arm -M stm32nucleo-l4r5zi -kernel build/Osiris.elf -nographic -semihosting --semihosting-config enable=on,target=native -nographic -serial mon:stdio
