

# Check if all required variables are defined before proceeding.

if(NOT DEFINED APP_TARGET_NAME)
    message(FATAL_ERROR "APP_TARGET_NAME must be defined before including integration.cmake")
endif()

if(NOT DEFINED OSIRIS_SOURCE_DIR)
    message(FATAL_ERROR "OSIRIS_SOURCE_DIR must be defined before including integration.cmake")
endif()

if(NOT DEFINED MCU)
    message(FATAL_ERROR "MCU must be defined before including integration.cmake")
endif()

if(NOT DEFINED BOARD)
    message(FATAL_ERROR "BOARD must be defined before including integration.cmake")
endif()

# Add osiris as a subdirectory.
add_subdirectory(${OSIRIS_SOURCE_DIR})

# Now we link osiris and all the object libraries to our app target.
# We sadly cannot link the object libraries in osiris directly as they do not propagate to the app target.
target_link_libraries(${APP_TARGET_NAME} PRIVATE
    osiris
    startup_${CPU}
    startup_${MCU}
)

# Set our linker script.
set_target_properties(${APP_TARGET_NAME} PROPERTIES
    LINK_DEPENDS "${LINKER_SCRIPT}"
    OUTPUT_NAME "${APP_TARGET_NAME}"
    SUFFIX ".elf"
)

#Compiler options.
target_compile_options(${APP_TARGET_NAME}
    PRIVATE
        ${COMPILE_FLAGS_${CPU}}
        -Wall
        -Wextra
        "-fstack-usage"
)

# Linker options.
target_link_options(${APP_TARGET_NAME}
    PRIVATE
        ${LINK_FLAGS_${CPU}}
        "-T${OSIRIS_LINKER_SCRIPT}"
        "-nostartfiles"
        "-nostdlib"
        "-fstack-usage"
)

# Create our symbols.map file.
add_custom_command(TARGET ${APP_TARGET_NAME} POST_BUILD
    COMMAND ${CMAKE_NM} --defined-only --demangle $<TARGET_FILE:${APP_TARGET_NAME}> > ${CMAKE_SOURCE_DIR}/symbols.map
    COMMENT "Generating symbols map symbols.map"
)

# If we have runtime symbols enabled, we need to inject them into the binary.
if (CONFIG_RUNTIME_SYMBOLS)
    # Post build symbol injection
    add_subdirectory(scripts/syms)

    # The syms script environment must be set up before we can use it. e.g. create venv and install dependencies.
    add_dependencies(${APP_TARGET_NAME} syms)

    add_custom_command(TARGET ${APP_TARGET_NAME} POST_BUILD
        COMMAND ${SYMS_CMD} -f $<TARGET_FILE:${APP_TARGET_NAME}>
        COMMENT "Injecting symbols into ${APP_TARGET_NAME}.elf"
    )
endif()

# Generate a binary file from the ELF file.
# This must happen after the syms injection, so we can get the symbols in the binary.
add_custom_command(TARGET ${APP_TARGET_NAME} POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -O binary $<TARGET_FILE:${APP_TARGET_NAME}> ${APP_TARGET_NAME}.bin
    COMMENT "Generating binary ${APP_TARGET_NAME}.bin"
)