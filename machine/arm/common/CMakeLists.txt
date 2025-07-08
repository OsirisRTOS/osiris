# machine/arm/cortex-m4/CMakeLists.txt

cmake_minimum_required(VERSION 3.28)

set(CMAKE_TOOLCHAIN_FILE ${CMAKE_CURRENT_SOURCE_DIR}/../cmake/toolchain.cmake)

project(STARTUP LANGUAGES C ASM)

set(CMAKE_VERBOSE_MAKEFILE ON)

set_property(SOURCE ivt.S APPEND PROPERTY COMPILE_OPTIONS "-x" "assembler-with-cpp")
set_property(SOURCE irq.S APPEND PROPERTY COMPILE_OPTIONS "-x" "assembler-with-cpp")

add_library(common STATIC
    ivt.S
    entry.c
    syscall.c
    irq.S
)

target_include_directories(common PUBLIC
    ../../../kernel/include
    ${CMAKE_CURRENT_SOURCE_DIR}
)

install(TARGETS common DESTINATION .)

