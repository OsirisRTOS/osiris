# machine/arm/cortex-m4/CMakeLists.txt

set(CPU_COMPILE_FLAGS
    -mcpu=${CPU}
    -mthumb
)

set(CPU_LINK_FLAGS
    -mcpu=${CPU}
    -mthumb
)

set_property(SOURCE crt0.s APPEND PROPERTY COMPILE_OPTIONS "-x" "assembler-with-cpp")

add_library(startup_${CPU} OBJECT
    crt0.S
    entry.c
)

target_include_directories(startup_${CPU}
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_options(startup_${CPU}
    PRIVATE
        ${CPU_COMPILE_FLAGS}
)

# Provide CPU-specific compiler and linker flags to parent scope
set(COMPILE_FLAGS_${CPU} "${CPU_COMPILE_FLAGS}" PARENT_SCOPE)
set(LINK_FLAGS_${CPU} "${CPU_LINK_FLAGS}" PARENT_SCOPE)

# Set the correct rust target for our cpu
set(Rust_CARGO_TARGET thumbv7em-none-eabi PARENT_SCOPE)

