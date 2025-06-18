
cmake_minimum_required(VERSION 3.28)
set(CMAKE_EXPORT_COMPILE_COMMANDS OFF)

set(CMAKE_VERBOSE_MAKEFILE ON)

set(MCU "stm32l4r5xx" CACHE STRING "target mcu")
set(BOARD "nucleo" CACHE STRING "target board")

set(MCU_CPU_ARCH_HAL_MAP
    "stm32l4r5xx cortex-m4 arm stm32l4xx"
)

# Extract MCU, CPU, HAL and ARCH from the MCU_CPU_ARCH_HAL_MAP
foreach(entry ${MCU_CPU_ARCH_HAL_MAP})
    string(REPLACE " " ";" entry_parts ${entry})
    list(GET entry_parts 0 mcu)

    if(NOT mcu STREQUAL MCU)
        continue()
    endif()

    list(GET entry_parts 1 cpu)
    list(GET entry_parts 2 arch)
    list(GET entry_parts 3 hal)

    set(CPU ${cpu})
    set(ARCH ${arch})
    set(HAL ${hal})

    message(STATUS "Build for MCU '${MCU}' with CPU '${CPU}', ARCH '${ARCH}', HAL '${HAL}' and BOARD '${BOARD}'")
endforeach()

# Check if CPU is set
if(CPU STREQUAL "cortex-m4")
    set(Rust_CARGO_TARGET thumbv7em-none-eabi)
else()
    message(FATAL_ERROR "Invalid MCU '${MCU}' specified. Please check the MCU_CPU_ARCH_HAL_MAP.")
endif()

if (ARCH STREQUAL "arm")
    set(CMAKE_TOOLCHAIN_FILE "${OSIRIS_SOURCE_DIR}/cmake/arm-none-eabi.cmake")
else()
    message(FATAL_ERROR "Unsupported architecture '${ARCH}'.")
endif()

# Enable debug symbols if a debug build is requested.
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CONFIG_RUNTIME_SYMBOLS ON)
endif()
