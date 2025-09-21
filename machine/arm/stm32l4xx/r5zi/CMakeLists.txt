cmake_minimum_required(VERSION 3.28)

add_library(chip_native STATIC lib.c)

target_include_directories(chip_native PUBLIC
    ../../../../kernel/include
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_definitions(chip_native PUBLIC
    STM32L4R5xx
)

set(TARGET_DEFINES "STM32L4R5xx" PARENT_SCOPE)