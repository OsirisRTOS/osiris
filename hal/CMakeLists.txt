# hal/CMakeLists.txt

if (${BOARD} MATCHES "stm32.*l4")
    message(STATUS "Detected hal 'stm32l4'.")
    set(HAL_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/stm32l4/${BOARD}")

# Add new hal's here.
else()
    message(FATAL_ERROR "Board '${BOARD}' not supported.")
endif()

if (NOT EXISTS "${HAL_SOURCE_DIR}")
    message(FATAL_ERROR "HAL does not support board '${BOARD}'.")
endif()

# Add the Hal library.
add_subdirectory(${HAL_SOURCE_DIR})