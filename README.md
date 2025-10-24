
# Osiris

[![CI](https://github.com/OsirisRTOS/osiris/actions/workflows/ci.yml/badge.svg)](https://github.com/OsirisRTOS/osiris/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

An RTOS designed and verified to enable reliable software updates and operation for embedded systems.


## Project Structure

| Directory | Description |
|-----------|-------------|
| [kernel/](kernel/) | This is the actual kernel of osiris. It is a hardware independent layer providing scheduling, memory management, etc. |
| [machine/](machine/) | This contains all the HALs and hardware specific code in general. It exports a hardware independent interface to the kernel. |
| [hal/](hal/) | Hardware Abstraction Layer implementations (cortex-m, stm32l4). |
| [tools/](tools/) | Development tools including configuration utilities and ELF injector. |
| [xtask/](xtask/) | Custom cargo xtask implementations for build automation. |
| [presets/](presets/) | Pre-configured build presets for different target boards (e.g., STM32L4R5ZI). |
| [verus/](verus/) | Verification-related files and components. |
| [.devcontainer/](.devcontainer/) | Docker-based development environment configuration. |
| [.github/](.github/) | CI/CD pipeline configurations and GitHub workflows. |

## Build

### Build Dependencies
*   **Rust**: A recent version of the Rust toolchain.
*   **Just**: A command runner, used for all project tasks.
*   **ARM Toolchain**: `arm-none-eabi-gcc` (version 13+ recommended).
*   **CMake**: Version 3.28 or newer.
*   **Clang**: Used as the C/C++ compiler.
*   **Python**: Version 3.12 or newer, with `pip` and `venv`.
*   **pyelftools**: For injecting runtime symbols into the ELF file.
*   **Kani**: A recent version of the Kani Rust Verifier.

### Development & Debugging Tools
These tools are used for flashing, debugging, and other development tasks.
*   `stlink`: For flashing and debugging on STM32 hardware.
*   `arm-none-eabi-gdb`: The GNU debugger for ARM targets.
*   `cargo-binutils`: Provides `objdump`, `objcopy`, etc. for Rust.

### Quick Start

#### **Development Environment**

##### Using DevContainer (Recommended)
The easiest way to get started is using the provided DevContainer, which includes all necessary dependencies:

1. Install [Docker](https://www.docker.com/) and [Visual Studio Code](https://code.visualstudio.com/)
2. Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
3. Open the repository in VS Code
4. When prompted, click "Reopen in Container" (or use Command Palette: "Dev Containers: Reopen in Container")

The DevContainer includes:
- Rust toolchain with embedded targets
- ARM GCC toolchain
- Kani verifier for formal verification
- All build tools (just, cmake, clang)
- Debugging tools (gdb, stlink)
- Coverage tools (tarpaulin)
- QEMU for emulation

##### Manual Setup
If you prefer not to use DevContainer, ensure you have all the dependencies listed in the [Build Dependencies](#build-dependencies) section installed on your system.

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
Build the kernel for your target architecture. The target triple selects the top-level HAL (e.g., ARM). Select the specific machine HAL via the configuration tool.

```sh
$ just build <target-triple>
```

After the build a binary named ```Kernel.bin``` will be created at the source root folder.

### Set up pre-commit hooks

```sh
$ just hooks
```

## Testing

### Running Tests
Run the test suite using:

```sh
$ just test
```

### Code Coverage
Generate test coverage reports using [cargo-tarpaulin](https://github.com/xd009642/tarpaulin):

```sh
$ just cov
```

This generates an `lcov.info` file that can be viewed with coverage visualization tools. The DevContainer includes the Coverage Gutters VS Code extension for inline coverage display.

### Formal Verification with Kani
Osiris uses [Kani](https://github.com/model-checking/kani) for formal verification of critical code paths:

```sh
$ just verify
```

Kani performs bounded model checking to mathematically prove the absence of certain classes of bugs, including:
- Arithmetic overflows/underflows
- Array out-of-bounds access
- Undefined behavior
- Assertion violations

The verification runs are also part of the CI pipeline to ensure all proofs pass on every commit.

## Continuous Integration

The project uses GitHub Actions for continuous integration. The CI pipeline includes:

- **Container Build**: Builds and caches the development container
- **Testing**: Runs the complete test suite with coverage reporting
- **Formatting**: Checks code formatting with `rustfmt`
- **Kani Verification**: Runs formal verification proofs
- **Target Builds**: Builds for specific hardware targets (e.g., STM32 Nucleo L4R5ZI)

You can view the current build status and detailed results in the [Actions tab](https://github.com/OsirisRTOS/osiris/actions).

## License

Osiris is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
