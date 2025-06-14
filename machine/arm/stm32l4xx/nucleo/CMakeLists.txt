cmake_minimum_required(VERSION 3.13)

# Glob all source files
file(GLOB_RECURSE SRC_FILES
    *.c
)

# Add the library target
add_library(nucleo_stm32l4xx STATIC ${SRC_FILES})

# Set the include directories so the cmake src root machine/arm/cmsis machine/arm/device/stm32l4xx
target_include_directories(nucleo_stm32l4xx PUBLIC
    ${CMAKE_SOURCE_DIR}/machine/arm/cmsis
    ${CMAKE_SOURCE_DIR}/machine/arm/stm32l4xx/device
    ${CMAKE_SOURCE_DIR}/machine/arm/stm32l4xx/hal
    ${CMAKE_CURRENT_SOURCE_DIR}
)

# Make ${MCU} uppercase except for the xx at the end
string(TOUPPER ${MCU} MCU_DEFINE)
# Remove the last two characters (xx) from MCU_DEFINE
string(LENGTH ${MCU_DEFINE} MCU_DEFINE_LENGTH)
math(EXPR MCU_DEFINE_LENGTH "${MCU_DEFINE_LENGTH} - 2")
string(SUBSTRING ${MCU_DEFINE} 0 ${MCU_DEFINE_LENGTH} MCU_DEFINE)
# Append "xx" to the MCU_DEFINE
string(CONCAT MCU_DEFINE "${MCU_DEFINE}xx")

target_compile_definitions(nucleo_stm32l4xx PUBLIC
    ${MCU_DEFINE}
)

target_compile_options(nucleo_stm32l4xx
    PRIVATE
        ${COMPILE_FLAGS_${CPU}}
)