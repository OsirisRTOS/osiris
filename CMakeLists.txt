# ./CMakeLists.txt

cmake_minimum_required(VERSION 3.28)
set(CMAKE_EXPORT_COMPILE_COMMANDS OFF)

set(CMAKE_VERBOSE_MAKEFILE OFF)

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
    set(CMAKE_TOOLCHAIN_FILE "${CMAKE_SOURCE_DIR}/cmake/arm-none-eabi.cmake")
else()
    message(FATAL_ERROR "Unsupported architecture '${ARCH}'.")
endif()

# Enable debug symbols if a debug build is requested.
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CONFIG_RUNTIME_SYMBOLS ON)
endif()

project(Osiris LANGUAGES C CXX ASM)

# Define the necessary scripts.
set(STARTUP_DIR "${CMAKE_SOURCE_DIR}/machine/startup/${CPU}")
set(LINKER_SCRIPT "${CMAKE_BINARY_DIR}/machine/${ARCH}/${HAL}/link.ld")

include_directories(
    ${CMAKE_SOURCE_DIR}/include
)

# Now add our startup code and the nlib.
# This will also set the compiler and linker flags.
add_subdirectory(machine/startup/${CPU})

# This sets the HAL for the kernel.
set(HAL_DIR "${CMAKE_SOURCE_DIR}/machine/${ARCH}")

# Add the C hal
add_subdirectory(${HAL_DIR})

add_subdirectory(kernel)

# Add the nlib and the board-specific code.
add_subdirectory(nlib)
add_subdirectory(machine/startup/mcu/${MCU})

add_executable(${PROJECT_NAME} 
    bootstrap.c
)

add_dependencies(startup_${CPU} syscalls_map)

target_link_libraries(kernel INTERFACE 
    ${BOARD}_${HAL}
)

target_link_libraries(${PROJECT_NAME}
    PRIVATE
        kernel
        nlib
        $<TARGET_OBJECTS:startup_${CPU}>
        $<TARGET_OBJECTS:startup_${MCU}>
)

set_target_properties(${PROJECT_NAME} PROPERTIES
    LINK_DEPENDS "${LINKER_SCRIPT}"
    OUTPUT_NAME "${PROJECT_NAME}"
    SUFFIX ".elf"
)

#Compiler options.
target_compile_options(${PROJECT_NAME}
    PRIVATE
        ${COMPILE_FLAGS_${CPU}}
        -Wall
        -Wextra
        "-fstack-usage"
)

# Linker options.
target_link_options(${PROJECT_NAME}
    PRIVATE
        ${LINK_FLAGS_${CPU}}
        "-T${LINKER_SCRIPT}"
        "-nostartfiles"
        "-nostdlib"
        "-fstack-usage"
)

add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
    COMMAND ${CMAKE_NM} --defined-only --demangle $<TARGET_FILE:${PROJECT_NAME}> > ${CMAKE_SOURCE_DIR}/symbols.map
    COMMENT "Generating symbols map symbols.map"
)

if (CONFIG_RUNTIME_SYMBOLS)
    # Post build symbol injection
    add_subdirectory(scripts/syms)

    # The syms script environment must be set up before we can use it. e.g. create venv and install dependencies.
    add_dependencies(${PROJECT_NAME} syms)

    add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
        COMMAND ${SYMS_CMD} -f $<TARGET_FILE:${PROJECT_NAME}>
        COMMENT "Injecting symbols into ${PROJECT_NAME}.elf"
    )
endif()

# This must happen after the syms injection, so we can get the symbols in the binary.
add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -O binary $<TARGET_FILE:${PROJECT_NAME}> ${PROJECT_NAME}.bin
    COMMENT "Generating binary ${PROJECT_NAME}.bin"
)
