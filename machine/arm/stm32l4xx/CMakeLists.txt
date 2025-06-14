# HAL
add_subdirectory(hal)
add_subdirectory(device)

# Put link.ld.in through the C preprocessor to generate link.ld
# Add CONFIG_DEFINES to the preprocessor command
add_custom_command(
  OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/link.ld
  COMMAND ${CMAKE_C_COMPILER} -E -P -x c ${CMAKE_CURRENT_SOURCE_DIR}/link.ld -o ${CMAKE_CURRENT_BINARY_DIR}/link.ld ${CONFIG_DEFINES}
  DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/link.ld
  COMMENT "Generating link.ld from link.ld.in"
)

add_custom_target(link_ld ALL DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/link.ld)

add_dependencies(hal_${HAL} link_ld)
target_link_libraries(hal_${HAL} PUBLIC device_${HAL})

# Boards
add_subdirectory(nucleo)
target_link_libraries(nucleo_${HAL} PUBLIC hal_${HAL})
