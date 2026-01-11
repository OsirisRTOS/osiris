cmake_minimum_required(VERSION 3.28)

add_library(variant_native STATIC lib.c)

target_include_directories(variant_native PUBLIC
    ../../../../include
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_definitions(variant_native PUBLIC
    STM32L4R5xx
)

set(TARGET_DEFINES "STM32L4R5xx" PARENT_SCOPE)