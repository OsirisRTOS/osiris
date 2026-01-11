build *args:
    cargo build {{args}}
    cargo xtask --release injector Cargo.toml
    cargo objcopy {{args}} -- -O binary Kernel.bin

config *args:
    cargo xtask config --root {{justfile_directory()}} {{args}}

fmt *args:
    cargo fmt {{args}}

verify *args:
    cargo kani -Z concrete-playback --concrete-playback=print -Z stubbing {{args}}

test *args:
    cargo test {{args}}

cov *args:
    cargo tarpaulin --out Lcov --skip-clean --engine llvm {{args}}

clean:
    cargo clean
    rm -f Kernel.bin

hooks:
    ln -sf {{justfile_directory()}}/.devcontainer/pre-commit.sh {{justfile_directory()}}/.git/hooks/pre-commit
