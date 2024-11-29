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

# Check if all the files exist.
if(NOT EXISTS "${STARTUP_DIR}/crt0.S")
    message(FATAL_ERROR "Startup file crt0.S not found in ${STARTUP_DIR}")
endif()

if(NOT EXISTS "${STARTUP_DIR}/entry.c")
    message(FATAL_ERROR "Startup file entry.c not found in ${STARTUP_DIR}")
endif()

if(NOT EXISTS "${LINKER_SCRIPT}")
    message(FATAL_ERROR "Linker script not found: ${LINKER_SCRIPT}")
endif()

include_directories(
    ${CMAKE_SOURCE_DIR}/include
)

# Now add our startup code and the nlib.
add_subdirectory(nlib)
add_subdirectory(machine/${CPU})

# Retrieve CPU-specific flags.
if(DEFINED COMPILE_FLAGS_${CPU})
    set(CPU_COMPILE_FLAGS ${COMPILE_FLAGS_${CPU}})
else()
    message(FATAL_ERROR "Compilation flags for CPU '${CPU}' not defined.")
endif()

if(DEFINED LINK_FLAGS_${CPU})
    set(CPU_LINK_FLAGS ${LINK_FLAGS_${CPU}})
else()
    message(FATAL_ERROR "Linking flags for CPU '${CPU}' not defined.")
endif()


add_executable(${PROJECT_NAME} 
    main.c
)


target_link_libraries(${PROJECT_NAME}
    PRIVATE
        nlib
        $<TARGET_OBJECTS:startup_${CPU}>
)

target_link_options(${PROJECT_NAME}
    PRIVATE
        "-T${LINKER_SCRIPT}"
)

set_target_properties(${PROJECT_NAME} PROPERTIES
    LINK_DEPENDS "${LINKER_SCRIPT}"
    OUTPUT_NAME "${PROJECT_NAME}"
    SUFFIX ".elf"
)

#Compiler options.
target_compile_options(${PROJECT_NAME}
    PRIVATE
        ${CPU_COMPILE_FLAGS}
        -Wall
        -Wextra
)

# Linker options.
target_link_options(${PROJECT_NAME}
    PRIVATE
        ${CPU_LINK_FLAGS}
        "-T${LINKER_SCRIPT}"
        "-nostartfiles"
        "-nostdlib"
)

add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -O binary $<TARGET_FILE:${PROJECT_NAME}> ${PROJECT_NAME}.bin
    COMMENT "Generating binary ${PROJECT_NAME}.bin"
)