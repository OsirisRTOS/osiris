# machine/board/stm32-nucleo-l4r5zi/CMakeLists.txt

add_library(startup_${BOARD} OBJECT
    isr_ext.c
    jumper.c
)

target_include_directories(startup_${BOARD}
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_options(startup_${BOARD}
    PRIVATE
        ${COMPILE_FLAGS_${CPU}}
)
