build target:
    cargo objcopy --target {{target}} -- -O binary Kernel.bin
    cargo xtask inject-syms --target {{target}}

release target:
    cargo objcopy --target {{target}} --release -- -O binary Kernel.bin
    cargo xtask inject-syms --target {{target}} --release

config:
    cargo run -p config

