cmake_minimum_required(VERSION 3.13)

# Glob all source files
file(GLOB_RECURSE SRC_FILES
    *.c
)

# Add the library target
add_library(device_native STATIC ${SRC_FILES})

# Set the include directories so the cmake src root machine/arm/cmsis machine/arm/device/stm32l4xx
target_include_directories(device_native PUBLIC
    ../../cmsis
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_definitions(device_native PUBLIC
    ${TARGET_DEFINES}
)