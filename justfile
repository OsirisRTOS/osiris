build *args:
    cargo build {{args}}
    cargo xtask --release injector Cargo.toml

config *args:
    cargo xtask config --root {{justfile_directory()}} {{args}}

example name *args: (build args)
    cargo build -p {{name}} {{args}}

fmt *args:
    cargo fmt {{args}}

verify *args:
    # This is giga hacky. But we need it until kani updates to the next version of cargo.
    OSIRIS_STACKPAGES=1 cargo kani -Z concrete-playback --concrete-playback=print -Z stubbing {{args}}

test *args:
    cargo test {{args}}

cov *args:
    cargo tarpaulin --out Lcov --skip-clean --engine llvm {{args}}

clean:
    cargo clean
    rm -f Kernel.bin

hooks:
    ln -sf {{justfile_directory()}}/.devcontainer/pre-commit.sh {{justfile_directory()}}/.git/hooks/pre-commit
