# HAL
add_subdirectory(hal)
add_subdirectory(device)

target_link_libraries(hal_${HAL} PUBLIC device_${HAL})

# Boards
add_subdirectory(nucleo)
target_link_libraries(nucleo_${HAL} PUBLIC hal_${HAL})
