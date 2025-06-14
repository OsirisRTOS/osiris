# stm32l4xx HAL

This is the hardware abstraction layer used for the stm32l4 family of microcontrollers.

## Project structure

### Third-Party HAL libraries

| Directory              | Autor              | License                                      | Description                                                                       |
|------------------------|--------------------|----------------------------------------------|-----------------------------------------------------------------------------------|
| [device/](device/)     | STMicroelectronics | [device/LICENSE.md](device/LICENSE.md)       | These are source/header files of the [STM32CubeL4 CMSIS Device MCU Component](https://github.com/STMicroelectronics/cmsis-device-l4) repository.    |
| [hal/](hal/)           | STMicroelectronics | [hal/LICENSE.md](hal/LICENSE.md)             | These are source/header files of the [STM32CubeL4 HAL Driver MCU Component](https://github.com/STMicroelectronics/stm32l4xx-hal-driver) repository. |
| [../cmsis/](../cmsis/) | ARM                | [../cmsis/LICENSE.txt](../cmsis/LICENSE.txt) | These are the core cmsis header files of the [CMSIS Version 5](https://github.com/ARM-software/CMSIS_5) repository.                                 |

### Board specific instantiations

| Directory          | Description                                                                                                                                                |
|--------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [nucleo/](nucleo/) | This uses bindings to the hal ([hal/](hal/) to define board specific instantiations of common functions used by the kernel for the nucleo dev boards by STM. |
| [config/](config/) | This defines memory maps and configuration options to be applied to the actual linker script. The respective file will be included (through config.ldconf) at the beginning of the linker script (link.ld) defined in the root of this hal. |

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

## How does the build process of this HAL work?

First each component with C source files so [hal/](hal/), [device/](device/) and the board folders will get compiled into a static library.
Then the ```build.rs``` file runs which generates bindings to these C source files based on the headers provided by the respective libraries.
This will generate two files ```bindings.rs``` and ```macros.rs``` (they will be placed in ```//build```), which get included in our ```lib.rs``` file.
Then the actual crate will be build as an rlib which then get's included in the kernel.

The CMakeLists.txt file serves as the bridge between CMake and Cargo, it will supply build.rs with the correct settings (Which MCU to build etc.) to generate the bindings.
Furthermore a linker script will be generated, based on [link.ld](link.ld). [link.ld](link.ld) will be processed by a C preprocessor which then generates the actual linker script (in the respective CMake binary directory). Defines like ```CONFIG_RUNTIME_SYMBOLS``` will be automatically supplied to the preprocessor based on the CMake configuration.