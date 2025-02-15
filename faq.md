

# Random bugs and stuff we encountered.

## My OS jumps to 0x1fffxxxxx during boot.
The embedded bootloader is executed for some reason.
Check if `st-flash --area=option read` reports `ffeff8aa`.
And set it if not `st-flash --area=option write 0xfbeff8aa`

## My OS crashes when ran without debugger
Check if you try to use semihosting for some reason. This will literally pause the cpu indefinitely.

## Debugger does not reset when HW button is pressed.
Write `c` in GDB first and then press the HW button.

## Get stlink working in Windows + WSL2.
```sh
$ winget install usbipd
$ usbipd list # select the STM32 device. Reopen terminal if command not found.
$ usbipd bind --busid=<BUSID> # execute as admin
$ usbipd attach --wsl --busid=<BUSID>
$ st-info --probe # inside WSL2 will now work.
```


