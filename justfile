build *args:
    cargo build {{args}}
    cargo xtask --release injector Cargo.toml

config *args:
    cargo xtask config --root {{justfile_directory()}} {{args}}

example name *args: (build args)
    cargo build -p {{name}} {{args}}
    cargo xtask --release injector Cargo.toml
    # TODO: This does override the injector binary
    # Post build steps should be target specific, this is just a temporary hack
    cargo objcopy -p {{name}} {{args}} -- -O binary {{name}}.bin

fmt *args:
    cargo fmt {{args}}

verify *args:
    # This is a temporary hack.
    OSIRIS_STACKPAGES=1 OSIRIS_MACHINE=cortex-m RUSTFLAGS="-Zcrate-attr=feature(cfg_select)" cargo kani -Z concrete-playback --concrete-playback=print -Z stubbing {{args}}

test *args:
    cargo test --target host-tuple {{args}}

cov *args:
    cargo tarpaulin --out Lcov --skip-clean --engine llvm {{args}}

docs *args:
    cargo doc --document-private-items --no-deps {{args}}

clean:
    cargo clean
    rm -f Kernel.bin

hooks:
    ln -sf {{justfile_directory()}}/.devcontainer/pre-commit.sh {{justfile_directory()}}/.git/hooks/pre-commit
