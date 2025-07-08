cmake_minimum_required(VERSION 3.13)

# Glob all source files
file(GLOB_RECURSE SRC_FILES
    *.c
)

# Exclude all files that contain template in the name
foreach(SRC_FILE ${SRC_FILES})
    if (SRC_FILE MATCHES "template")
        list(REMOVE_ITEM SRC_FILES ${SRC_FILE})
    endif()
endforeach()

# Add the library target
add_library(hal_native STATIC ${SRC_FILES})

# Set the include directories so the cmake src root machine/arm/cmsis machine/arm/device/stm32l4xx
target_include_directories(hal_native PUBLIC
    ../../cmsis
    ../device
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_definitions(hal_native PUBLIC
    ${TARGET_DEFINES}
)