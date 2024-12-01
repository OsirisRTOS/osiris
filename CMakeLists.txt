# kernel/CMakeLists.txt

include(FetchContent)

# Path to your Cargo.toml file
set(CARGO_TOML_PATH "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml.in")

configure_file(${CARGO_TOML_PATH} "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" @ONLY)

FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH Cargo.toml NO_STD)

target_sources(kernel 
    INTERFACE
        FILE_SET HEADERS 
        BASE_DIRS include
        FILES
            include/kernel/lib.h
)

set(RUST_KERNEL true PARENT_SCOPE)