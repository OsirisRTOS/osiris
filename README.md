
# Osiris
An RTOS designed and verified to enable reliable software updates and operation for embedded systems.


## Project Structure

| Directory | Description |
|-----------|-------------|
| [src/](src/) | This is the actual kernel code of osiris. It is a hardware independent layer providing scheduling, memory management, etc. |
| [machine/](machine/) | This contains all the HALs and hardware specific code in general. It exports a hardware independent interface to the kernel. |

## Build

### Build Dependencies
*   **Rust**: A recent version of the Rust toolchain.
*   **Just**: A command runner, used for all project tasks.
*   **ARM Toolchain**: `arm-none-eabi-gcc` (version 13+ recommended).
*   **CMake**: Version 3.28 or newer.
*   **Clang**: Used as the C/C++ compiler.
*   **Kani**: A recent version of the Kani Rust Verifier.

### Development & Debugging Tools
These tools are used for flashing, debugging, and other development tasks.
*   [stlink (Fork)](https://github.com/CreaxxOG/stlink): For flashing and debugging on STM32 hardware.
*   `arm-none-eabi-gdb`: The GNU debugger for ARM targets.
*   `cargo-binutils`: Provides `objdump`, `objcopy`, etc. for Rust.

### Quick Start

#### **Configure the build.**  
Configure all build components. The configuration is stored in `.cargo/config.toml` as environment variables with the `OSIRIS_` prefix.

```sh
$ just config
```

or load a preset via:

```sh
$ just config load <preset_name> [--no-confirm]
```

if you want to reset the configuration run:

```sh
$ just config clean [--no-confirm]
```

#### **Build the kernel.** 
This will build the kernel for your current config.

```sh
$ just build
```

#### **Build an example app.**
In order to build an example run ```just example <name>```.
For example:
```sh
$ just example hello-world
```

After the build a binary named ```hello-world.bin` will be created at the source root folder. This can then be flashed onto the MCU.

### Set up pre-commit hooks

```sh
$ just hooks
```

## License

Osiris is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

## Contributing

Read the [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a pull request or otherwise contributing to this repository.

## Contact

Osiris was created and is maintained by Thomas Wachter ([@thomasw04](https://github.com/thomasw04/)).
