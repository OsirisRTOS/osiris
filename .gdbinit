# --- Rust-friendly defaults ---
set language rust
set print pretty on
set print demangle on
set demangle-style rust
set pagination off
set confirm off
set disassemble-next-line on

# --- Auto-pick newest Kernel under target/ ---
python
import os, traceback

ROOT = os.getcwd()
TARGET_DIR = os.path.join(ROOT, "target")

def newest_kernel():
    cands = []
    for dirpath, _, filenames in os.walk(TARGET_DIR):
        for fn in filenames:
            if fn == "Kernel":  # only files named exactly Kernel
                p = os.path.join(dirpath, fn)
                if os.path.isfile(p):
                    cands.append(p)
    if not cands:
        return None
    cands.sort(key=lambda p: os.path.getmtime(p), reverse=True)
    return cands[0]

try:
    # Allow manual override via env var RUST_ELF
    elf = os.environ.get("RUST_ELF")
    if elf and not os.path.isabs(elf):
        elf = os.path.join(ROOT, elf)

    if elf and os.path.isfile(elf):
        chosen = elf
    else:
        chosen = newest_kernel()

    if not chosen:
        print("[gdbinit] No Kernel found under target/. Build first, or set RUST_ELF.")
    else:
        print(f"[gdbinit] Using ELF: {os.path.relpath(chosen, ROOT)}")
        gdb.execute(f"file {chosen}")

except Exception:
    print("[gdbinit] Error while choosing Kernel ELF:")
    traceback.print_exc()
end

target remote :4242
