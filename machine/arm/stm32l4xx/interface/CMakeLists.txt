cmake_minimum_required(VERSION 3.28)

# Glob all source files
file(GLOB_RECURSE SRC_FILES
    *.c
)

# Add the library target
add_library(interface_native STATIC ${SRC_FILES})

# Set the include directories so the cmake src root machine/arm/cmsis machine/arm/device/stm32l4xx
target_include_directories(interface_native PUBLIC
    ../../cmsis
    ../device
    ../hal
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_definitions(interface_native PUBLIC
    ${TARGET_DEFINES}
    BOARD_HEADER="../board/${BOARD}.h"
)