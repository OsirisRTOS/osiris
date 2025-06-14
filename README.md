# Osiris
An RTOS designed and verified to enable reliable software updates and operation for satellites and drones.


## Project Structure

| Directory | Description |
|-----------|-------------|
| [kernel/](kernel/) | This is the actual kernel of osiris. It is a hardware independent layer providing scheduling, memory management, etc. |
| [machine/arm](machine/arm) | This provides hardware abstraction layers (HAL) for all ARM based machines. Each folder (exception is common code) is named after the machine or family of machines that it provides the abstraction for. |
| [machine/startup](machine/startup/) | This provides the startup code for each cpu and board. |
| [nlib](nlib/) | This is a minimum C lib providing memcpy, memcmp and memset. |


## Build

### Dependencies

- cmake 3.28
- arm-none-eabi-gcc-13 (building with version 10 will fail)
- python 3.12
- pip
- venv

### Create build dir

```sh
$ mkdir build
$ cd build
```

### Build the project for the corresponding target
```sh
$ cmake -DBOARD=nucleo -DMCU=stm32l4r5xx ..
$ make
```

### Set up pre-commit hooks

```sh
$ make hooks
```

