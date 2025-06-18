# ./CMakeLists.txt

cmake_minimum_required(VERSION 3.28)

# Check if we are building as a library or an application.
if (NOT DEFINED APP_TARGET_NAME)
    message("Building osiris as a library, no app target defined.")

    # The prologue needs to know the OSIRIS_SOURCE_DIR, which is the current directory.
    set(OSIRIS_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR})
    include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/prologue.cmake)

    project(Osiris LANGUAGES C CXX ASM)
else()

    # If we are building an application, we need to export a linker script to our parent scope.
    set(OSIRIS_LINKER_SCRIPT "${CMAKE_CURRENT_BINARY_DIR}/machine/${ARCH}/${HAL}/link.ld" PARENT_SCOPE)
endif()

include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Now add our startup code and the nlib.
# This will also set the compiler and linker flags.
add_subdirectory(machine/startup/${CPU})

# This sets the HAL for the kernel.
set(HAL_DIR "${CMAKE_CURRENT_SOURCE_DIR}/machine/${ARCH}")

# Add the C hal
add_subdirectory(${HAL_DIR})

# Add our kernel target.
add_subdirectory(kernel)

# Add the nlib and the board-specific code.
add_subdirectory(nlib)
add_subdirectory(machine/startup/mcu/${MCU})

add_dependencies(startup_${CPU} syscalls_map)

add_library(osiris INTERFACE)

target_link_libraries(kernel INTERFACE 
    ${BOARD}_${HAL}
)

target_link_libraries(osiris INTERFACE
        kernel
        nlib
        # So this needs some explanation:
        # The startup_${CPU} and startup_${MCU} targets are object libraries, they only get linked into their "direct" parent target.
        # Thus this will not propagate to the app target. Why do we keep them here? Just to check if they compile on a standalone build (CI).
        # They will be actually linked by the integration.cmake script into the app target.
        startup_${CPU}
        startup_${MCU}
)