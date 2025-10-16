
cmake_minimum_required(VERSION 3.28)

set(CMAKE_TOOLCHAIN_FILE ${CMAKE_CURRENT_SOURCE_DIR}/../cmake/toolchain.cmake)

project(HAL C ASM)

set(CMAKE_VERBOSE_MAKEFILE ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Turn all cache variables starting with OSIRIS_ into compile definitions
get_cmake_property(_cache_vars CACHE_VARIABLES)
set(CONFIG_DEFINES "")
foreach(var ${_cache_vars})
  if(var MATCHES "^OSIRIS_")
    if(DEFINED ${var})
      # Quote value if it contains non-identifier chars
      if("${${var}}" MATCHES "^[0-9A-Za-z_]+$")
        add_compile_definitions(${var}=${${var}})
        list(APPEND CONFIG_DEFINES -D${var}=${${var}})
      else()
        # Escape quotes
        string(REPLACE "\"" "\\\"" _esc "${${var}}")
        add_compile_definitions(${var}=\"${_esc}\")
        list(APPEND CONFIG_DEFINES -D${var}=\"${_esc}\")
      endif()
      # Ensure rebuild if any value changes
      set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${var}")
    endif()
  endif()
endforeach()


# this will compile our chip_stm32l4xx library
add_subdirectory(${OSIRIS_ARM_STM32L4XX_MCU})

# HAL
add_subdirectory(hal)
add_subdirectory(device)

# Put link.ld through the C preprocessor
# Add CONFIG_DEFINES to the preprocessor command
set(LINKER_SCRIPT_IN  ${CMAKE_CURRENT_SOURCE_DIR}/link.ld)
set(LINKER_SCRIPT_OUT ${OUT_DIR}/link.ld)

# We track environment variables that start with OSIRIS_ as dependencies as they can change the output of the compilation
set(DEPS_FILE ${OUT_DIR}/deps.txt)
file(WRITE ${DEPS_FILE} "${CONFIG_DEFINES}")

add_custom_command(
  OUTPUT  ${LINKER_SCRIPT_OUT}
  COMMAND ${CMAKE_C_COMPILER} -E -P -x c
          ${CONFIG_DEFINES}
          -DMCU_HEADER=\"${OSIRIS_ARM_STM32L4XX_MCU}/link.ld\"
          ${LINKER_SCRIPT_IN}
          -o ${LINKER_SCRIPT_OUT}
  DEPENDS ${LINKER_SCRIPT_IN} ${DEPS_FILE}
  COMMENT "Generating link.ld"
  VERBATIM
)

add_custom_target(link_ld ALL DEPENDS ${LINKER_SCRIPT_OUT})

add_dependencies(hal_native link_ld)
target_link_libraries(hal_native PUBLIC device_native)

# Interface
add_subdirectory(interface)
target_link_libraries(interface_native PUBLIC hal_native chip_native)

install(TARGETS interface_native hal_native device_native chip_native DESTINATION .)
