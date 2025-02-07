# ./CMakeLists.txt

cmake_minimum_required(VERSION 3.28)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

set(BOARD "stm32-nucleo-l4r5zi" CACHE STRING "target board")
set(CPU "cortex-m4" CACHE STRING "target cpu")

set(COMBO_IS_VALID FALSE)

# The layout is "board cpu1,cpu2,...,cpuN"
set(BOARD_CPU_MAP 
    "stm32-nucleo-l4r5zi cortex-m4"
    ""
)

# Build a list of valid boards and CPUs
foreach(entry ${BOARD_CPU_MAP})
    string(REPLACE " " ";" entry_parts ${entry})
    list(GET entry_parts 0 board)

    if(NOT board STREQUAL BOARD)
        continue()
    endif()

    list(GET entry_parts 1 cpu_list)
    string(REPLACE "," ";" cpus ${cpu_list})

    foreach(cpu ${cpus})
        if(cpu STREQUAL CPU)
            set(COMBO_IS_VALID TRUE)
        endif()
    endforeach()

    if (NOT COMBO_IS_VALID)
        message(FATAL_ERROR "Invalid combination of BOARD='${BOARD}' and CPU='${CPU}'. Compatible CPUs for '${BOARD}' are: ${cpus}")
    else ()
        message(STATUS "Detected board '${BOARD}' with CPU '${CPU}'")
    endif()
endforeach()

if (NOT COMBO_IS_VALID)
    message(FATAL_ERROR "Board '${BOARD}' not supported.")
endif()

if(CPU MATCHES "cortex-m[0-9]+")
    set(CMAKE_TOOLCHAIN_FILE "${CMAKE_SOURCE_DIR}/cmake/arm-none-eabi.cmake")
else()
    message(FATAL_ERROR "Unsupported CPU: ${CPU}")
endif()

project(Osiris LANGUAGES C CXX ASM)

# Define the necessary scripts.
set(STARTUP_DIR "${CMAKE_SOURCE_DIR}/machine/${CPU}")
set(LINKER_SCRIPT "${CMAKE_SOURCE_DIR}/machine/board/${BOARD}/link.ld")

# Check if the linker script exists.
if(NOT EXISTS "${LINKER_SCRIPT}")
    message(FATAL_ERROR "Linker script not found: ${LINKER_SCRIPT}")
endif()

include_directories(
    ${CMAKE_SOURCE_DIR}/include
)

# Check if the kernel is a rust kernel.
if(EXISTS "${CMAKE_SOURCE_DIR}/kernel/Cargo.toml" OR EXISTS "${CMAKE_SOURCE_DIR}/kernel/Cargo.toml.in")
    set(RUST_KERNEL true)
endif()

add_subdirectory(hal)
add_subdirectory(kernel)

# Now add our startup code and the nlib.
# This will also set the compiler and linker flags.
add_subdirectory(machine/${CPU})

# Add the nlib and the board-specific code.
add_subdirectory(nlib)
add_subdirectory(machine/board/${BOARD})

add_executable(${PROJECT_NAME} 
    bootstrap.c
)

if (RUST_KERNEL)
    message(STATUS "Linking Rust kernel.")
    # A rust kernel does only support rust HALs and links it themselves.
    target_link_libraries(${PROJECT_NAME}
        PRIVATE
            $<TARGET_OBJECTS:startup_${CPU}>
            $<TARGET_OBJECTS:startup_${BOARD}>
            nlib
            kernel
    )
else()
    message(STATUS "Linking C kernel.")

    target_link_libraries(${PROJECT_NAME}
        PRIVATE
            $<TARGET_OBJECTS:startup_${CPU}>
            $<TARGET_OBJECTS:startup_${BOARD}>
            nlib
            hal
            kernel
    )
endif()

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
    COMMAND ${CMAKE_OBJCOPY} -O binary $<TARGET_FILE:${PROJECT_NAME}> ${PROJECT_NAME}.bin
    COMMENT "Generating binary ${PROJECT_NAME}.bin"
)