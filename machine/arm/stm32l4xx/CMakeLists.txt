

add_subdirectory(hal)
add_subdirectory(device)
add_subdirectory(macros)

target_link_libraries(hal_${HAL} PUBLIC device_${HAL} macros_${HAL})