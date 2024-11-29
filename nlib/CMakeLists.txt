# nlib/CMakeLists.txt

set(CPU_COMPILE_FLAGS
    -mcpu=${CPU}
    -mthumb
)

set(CPU_LINK_FLAGS
    -mcpu=${CPU}
    -mthumb
)

add_library(nlib STATIC
    core.c
)

target_include_directories(nlib
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_options(nlib
    PRIVATE
        ${CPU_COMPILE_FLAGS}
)
