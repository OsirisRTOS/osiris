build target:
    cargo build --target {{target}}
    cargo xtask inject-syms --target {{target}}
    cargo objcopy --target {{target}} -- -O binary Kernel.bin

release target:
    cargo build --target {{target}} --release
    cargo xtask inject-syms --target {{target}} --release
    cargo objcopy --target {{target}} --release -- -O binary Kernel.bin

config *args:
    cargo run -p config -- {{args}}

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
