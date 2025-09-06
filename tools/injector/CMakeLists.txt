cmake_minimum_required(VERSION 3.10)

# We need to run a python venv with a script in it
find_package(Python3 COMPONENTS Interpreter REQUIRED)

# Create a virtual environment if it doesn't exist
add_custom_command(
    OUTPUT .venv
    COMMAND ${Python3_EXECUTABLE} -m venv .venv
)

add_custom_command(
    OUTPUT venv.stamp
    DEPENDS .venv requirements.txt
    COMMAND ${CMAKE_COMMAND} -E copy ${CMAKE_CURRENT_SOURCE_DIR}/requirements.txt requirements.txt
    COMMAND ${CMAKE_CURRENT_BINARY_DIR}/.venv/bin/pip install -r requirements.txt --upgrade
)

add_custom_target(
    syms
    DEPENDS venv.stamp
)

set(SYMS_CMD ${CMAKE_CURRENT_BINARY_DIR}/.venv/bin/python ${CMAKE_CURRENT_SOURCE_DIR}/inject_syms.py PARENT_SCOPE)

