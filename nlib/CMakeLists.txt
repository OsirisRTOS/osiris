# nlib/CMakeLists.txt

add_library(nlib STATIC
    core.c
)

target_include_directories(nlib
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_options(nlib
    PRIVATE
        ${COMPILE_FLAGS_${CPU}}
)
