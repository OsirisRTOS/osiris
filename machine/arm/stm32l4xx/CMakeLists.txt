
cmake_minimum_required(VERSION 3.28)

set(CMAKE_TOOLCHAIN_FILE ${CMAKE_CURRENT_SOURCE_DIR}/../cmake/toolchain.cmake)

project(HAL C ASM)

set(CMAKE_VERBOSE_MAKEFILE ON)

# this will compile our chip_stm32l4xx library
add_subdirectory(${MCU})

# HAL
add_subdirectory(hal)
add_subdirectory(device)

# Put link.ld.in through the C preprocessor to generate link.ld
# Add CONFIG_DEFINES to the preprocessor command
add_custom_command(
  OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/link.ld
  COMMAND ${CMAKE_C_COMPILER} -E -P -x c ${CMAKE_CURRENT_SOURCE_DIR}/link.ld -o ${OUT_DIR}/link.ld -D${MCU} -DMCU_HEADER="\\\"${MCU}/link.ld\\\""
  DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/link.ld
  COMMENT "Generating link.ld from link.ld.in"
)

add_custom_target(link_ld ALL DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/link.ld)

add_dependencies(hal_native link_ld)
target_link_libraries(hal_native PUBLIC device_native)

# Interface
add_subdirectory(interface)
target_link_libraries(interface_native PUBLIC hal_native chip_native)

install(TARGETS interface_native hal_native device_native chip_native DESTINATION .)
