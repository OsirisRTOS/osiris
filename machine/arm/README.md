# machine/arm

This folder provides a hardware abstraction layers (HAL) for each ARM based machine that is supported.
There are folders for each family of machines with the underlying HALs.

## Common Code
| Directory | Description |
|-----------|-------------|
| [common/](common/) | This contains common interrupt, scheduling, startup, syscall code for ARM targets. | 

## Third-Party Common Code
| Directory | Author | License | Description |
|-----------|-------|---------|-------------|
| [cmsis/](cmsis/) | ARM                | [cmsis/LICENSE.txt](cmsis/LICENSE.txt) | These are the core cmsis header files of the [CMSIS Version 5](https://github.com/ARM-software/CMSIS_5) repository.                                 |

## Hardware Abstraction Layers
| Directory | Machine/Family | Description |
|-----------|----------------|-------------|
| [stm32l4xx](stm32l4xx/) | STM32L4xx | A HAL for the STM32L4xx family of machines. |


## How can a new HAL be added?

The build system expects the following things to be present in order to successfully build a HAL.

1.  **A CMake project** that builds these static libraries:
    *   `device_native`: CMSIS device files.
    *   `hal_native`: The vendor's HAL files.
    *   `interface_native`: Your C code that the kernel will call.
    *   `variant_native`: Variant-specific definitions like memory layout of the actual MCU used.

2.  **A linker script** named `link.ld` in your HAL's root directory (e.g., `machine/arm/new_hal/link.ld`).

3.  **An entry in `options.toml`** to make your new HAL selectable in the configuration tool.

Furthermore you can use all environment variables having the  ```OSIRIS_``` prefix as variables in your CMakeLists.txt to get information about the requested build.
After your new HAL is defined make sure you add it as possible configuration option to [options.toml](options.toml).

### What symbols does my HAL need to export?

The kernel calls C functions through a Rust module named `bindings`. Any function used in this module must be defined in your HAL.

The build system automatically generates these bindings from a header file named `interface/export.h` in your HAL directory. You must create this file and declare all exported functions in it.

## How does the build process of this HAL work?

The `build.rs` script finds the HAL folder defined via the `OSIRIS_ARM_HAL` environment variable. It builds the `CMakeLists.txt` project in that folder, forwarding all environment variables with the `OSIRIS_` prefix to CMake and setting up the correct target and properties. The script then generates bindings based on the `{HAL}/interface/export.h` header.  

**Note**: Each HAL can define its configuration options in its own `options.toml` file.

After the CMake build finishes, `build.rs` looks for a `link.ld` file in the `OUT_DIR` directory and forwards it to the hal-select (and kernel) crate.