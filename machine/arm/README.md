# machine/arm

This folder provides a hardware abstraction layers (HAL) for each ARM based machine that is supported.
There are folders for each family of machines with the underlying HALs.

## Third-Party Common Code
| Directory | Autor | License | Description |
|-----------|-------|---------|-------------|
| [cmsis/](cmsis/) | ARM                | [cmsis/LICENSE.txt](cmsis/LICENSE.txt) | These are the core cmsis header files of the [CMSIS Version 5](https://github.com/ARM-software/CMSIS_5) repository.                                 |

## Hardware Abstraction Layers
| Directory | Machine/Family | Description |
|-----------|----------------|-------------|
| [stm32l4xx](stm32l4xx/) | STM32L4xx | A HAL for the STM32L4xx family of machines. |


## How can a new HAL be added?

The build system expects the following things to be present in order to successfully build a HAL.

- A CMake target with the name ```${BOARD}_${HAL}```. This is the underlying HAL that will be linked.
- A linker script named ```link.ld``` in the binary directory of the HAL. e.g. ```//build/machine/arm/stm32l4xx/link.ld```

Furthermore you can use the variables ```${MCU}```, ```${BOARD}``` and ```${HAL}``` in your CMakeLists.txt to get information about the requested build.
After your new HAL is defined you just need to add it as a subdirectory to [CMakeLists.txt](CMakeLists.txt).

### What symbols does my HAL need to export?

Currently there is no exhaustive list of what is necessary to be exported. But every function used by the top-level HAL is referenced through the module ```bindings```
in the rust code. Every referenced symbol needs to be defined by the underlying HAL.

## What is important when modifying the top-level HAL?

In order for tests to run properly there is a "host" version needed for each function. So an implementation that enables this top-level HAL to be compiled on the host machine.
For that there is the so-called "host" feature, whenever this feature is enabled at compile time only the host compatible code has to be compiled.
