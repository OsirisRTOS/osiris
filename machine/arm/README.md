# machine/arm

This folder provides hardware abstraction layers (HAL) for each ARM based machine that is supported.
Additionally there is common code for ARM based machines in this folder:

## Third-Party Common Code
| Directory | Autor | License | Description |
|-----------|-------|---------|-------------|
| [cmsis/](cmsis/) | ARM                | [cmsis/LICENSE.txt](cmsis/LICENSE.txt) | These are the core cmsis header files of the [CMSIS Version 5](https://github.com/ARM-software/CMSIS_5) repository.                                 |

## Hardware Abstraction Layers
| Directory | Machine/Family | Description |
|-----------|----------------|-------------|
| [stm32l4xx](stm32l4xx/) | STM32L4xx | A HAL for the STM32L4xx family of machines. |
