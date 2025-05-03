# kernel/CMakeLists.txt

cmake_minimum_required(VERSION 3.23)

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
            include/kernel/syscalls.map.gen.h
)

# See https://doc.rust-lang.org/rustc/codegen-options/index.html
corrosion_add_target_rustflags(kernel "-Crelocation-model=ropi-rwpi" "-Crelro-level=off")

add_custom_target(syscalls_map DEPENDS cargo-build_kernel)