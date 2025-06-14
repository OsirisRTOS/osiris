

# Set the MCU define
set(CONFIG_DEFINES 
    -D${MCU}
)

# If CONFIG_RUNTIME_SYMBOLS is defined add it to the defines
if(CONFIG_RUNTIME_SYMBOLS)
    list(APPEND CONFIG_DEFINES -DCONFIG_RUNTIME_SYMBOLS)
endif()

add_subdirectory(stm32l4xx)