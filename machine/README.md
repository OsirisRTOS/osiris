# machine
This folder provides top-level HALs to the kernel, as well as the common interface that the kernel uses to interact with the HAL.

## Directory structure

| Directory | Description |
|-----------|-------------|
| [api/](api/) | This provides the api interface provided to the kernel. All hardware abstraction layers need to implement this. This will be implemented by the top-level HALs and re-exported via [select/](select/) |
| [arm/](arm/) | This provides a hardware abstraction layer (HAL) for all ARM-based machines. Each folder (exception is common/cmsis code) is named after the machine or family of machines that it provides the abstraction for. |
| [select/](select/) | This crate selects the correct hardware abstraction layer based on the architecture of the target. (e.g. ARM) This is what the kernel will include as dependency. |
| [testing/](testing/) | This is the hardware abstraction layer implementation when running tests or verification. It contains mostly stubs. |