# nlib/CMakeLists.txt

add_library(nlib STATIC
    src/core.c
)

target_include_directories(nlib
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_compile_options(nlib
    PRIVATE
        # Add compiler options specific to nlib if needed
)

target_compile_definitions(nlib
    PUBLIC
        # e.g., -DNLIB_CONFIG_OPTION
)
