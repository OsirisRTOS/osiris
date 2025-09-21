# machine/arm/cortex-m4/CMakeLists.txt

cmake_minimum_required(VERSION 3.28)

set(CMAKE_TOOLCHAIN_FILE ${CMAKE_CURRENT_SOURCE_DIR}/../cmake/toolchain.cmake)

project(STARTUP LANGUAGES C ASM)

set(CMAKE_VERBOSE_MAKEFILE ON)

# Turn all cache variables starting with OSIRIS_ into compile definitions
get_cmake_property(_cache_vars CACHE_VARIABLES)
foreach(var ${_cache_vars})
  if(var MATCHES "^OSIRIS_")
    if(DEFINED ${var})
      # Quote value if it contains non-identifier chars
      if("${${var}}" MATCHES "^[0-9A-Za-z_]+$")
        add_compile_definitions(${var}=${${var}})
      else()
        # Escape quotes
        string(REPLACE "\"" "\\\"" _esc "${${var}}")
        add_compile_definitions(${var}=\"${_esc}\")
      endif()
      # Ensure rebuild if any value changes
      set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${var}")
    endif()
  endif()
endforeach()

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

