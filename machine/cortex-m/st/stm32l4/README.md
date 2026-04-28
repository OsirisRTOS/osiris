# stm32l4xx HAL

This is the hardware abstraction layer used for the stm32l4 family of microcontrollers.

## Project structure

### Third-Party HAL libraries

| Directory              | Author              | License                                      | Description                                                                       |
|------------------------|--------------------|----------------------------------------------|-----------------------------------------------------------------------------------|
| [device/](device/)     | STMicroelectronics | [device/LICENSE.md](device/LICENSE.md)       | These are source/header files of the [STM32CubeL4 CMSIS Device MCU Component](https://github.com/STMicroelectronics/cmsis-device-l4) repository.    |
| [hal/](hal/)           | STMicroelectronics | [hal/LICENSE.md](hal/LICENSE.md)             | These are source/header files of the [STM32CubeL4 HAL Driver MCU Component](https://github.com/STMicroelectronics/stm32l4xx-hal-driver) repository. |
| [../cmsis/](../cmsis/) | ARM                | [../cmsis/LICENSE.txt](../cmsis/LICENSE.txt) | These are the core cmsis header files of the [CMSIS Version 5](https://github.com/ARM-software/CMSIS_5) repository.                                 |

### Exported bindings to the ARM HAL

| Directory          | Description                                                                                                                                                |
|--------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [interface](interface/) | This exports functions to be used by the more general ARM HAL through ```bindings```. |

### MCU specific instantiations

| Directory          | Description                                                                                                                                                |
|--------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [r5zi/](r5zi/) | This defines MCU specific things such as the memory layout which will be passed to the kernel. |

## How to update the third-party repositories?

### Device
Open the third-party repository.
1. Copy out all files from the ```//Include``` directory into the [device/](device/) directory.
2. Copy out the file ```//Source/Templates/system_stm32l4xx.c``` into the [device/](device/) directory.
3. Copy the ```//LICENSE.md``` file into the [device/](device/) directory.
4. Check if everything works.

### Hal
Open the third-party repository.
1. Copy out all files from the ```//Inc``` directory into the [hal/](hal/) directory.
2. Copy out all files from the ```//Src``` directory into the [hal/](hal/) directory.
3. Copy the ```//LICENSE.md``` file into the [hal/](hal/) directory.
4. Check if everything works.

### CMSIS
Open the third-party repository.
1. Copy out all files from the ```//CMSIS/Core/Include``` directory into the [../cmsis/](../cmsis/) directory.
2. Copy the ```//LICENSE.txt``` file into the [../cmsis/](../cmsis/) directory.
3. Check if everything works.