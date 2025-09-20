build target:
    cargo objcopy --target {{target}} -- -O binary Kernel.bin
    cargo xtask inject-syms --target {{target}}

release target:
    cargo objcopy --target {{target}} --release -- -O binary Kernel.bin
    cargo xtask inject-syms --target {{target}} --release

config:
    cargo run -p config

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
