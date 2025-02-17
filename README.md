# Osiris
An RTOS designed and verified to enable reliable software updates and operation for satellites and drones.

## Build

### Dependencies

- cmake 3.28
- arm-none-eabi-gcc

### Create build dir

```sh
$ mkdir build
$ cd build
```

### Build the project for the corresponding target
```sh
$ cmake -DBOARD=stm32-nucleo-l4r5zi -DCPU=cortex-m4 ..
$ make
```

### Set up pre-commit hooks

```sh
$ make hooks
```

## Boot

1. Install @thomasw04 qemu fork: https://github.com/thomasw04/qemu
2. Run ```./boot.sh```
3. Now you should see a: "Hello World!".
