set language rust
set print pretty on
set print demangle on
set demangle-style rust
set pagination off
set confirm off
set disassemble-next-line on

python
import os, traceback
import gdb

ROOT = os.getcwd()
TARGET_DIR = os.path.join(ROOT, "target")

def newest_kernel():
    cands = []
    for dirpath, _, filenames in os.walk(TARGET_DIR):
        for fn in filenames:
            if fn == "Kernel":
                p = os.path.join(dirpath, fn)
                if os.path.isfile(p):
                    cands.append(p)
    if not cands:
        return None
    cands.sort(key=lambda p: os.path.getmtime(p), reverse=True)
    return cands[0]

try:
    elf = os.environ.get("KERNEL_ELF")
    if elf and not os.path.isabs(elf):
        elf = os.path.join(ROOT, elf)

    if elf and os.path.isfile(elf):
        chosen = elf
    else:
        chosen = newest_kernel()

    if not chosen:
        print("[gdbinit] No Kernel found under target/. Build first, or set KERNEL_ELF.")
    else:
        print(f"[gdbinit] Using ELF: {os.path.relpath(chosen, ROOT)}")
        gdb.execute(f"file {chosen}")

except Exception:
    print("[gdbinit] Error while choosing Kernel ELF:")
    traceback.print_exc()

class Os(gdb.Command):
    """Osiris top-level command."""
    
    def __init__(self):
        super(Os, self).__init__("os", gdb.COMMAND_USER)

    def invoke(self, arg, from_tty):
        args = gdb.string_to_argv(arg)
        if len(args) == 0:
            self.print_help()
            return

        subcommand = args[0]
        subargs = args[1:]

        if subcommand == "hf":
            self.hfault(" ".join(subargs), from_tty)
        else:
            print(f"Unknown subcommand: {subcommand}")
            self.print_help()

    def print_help(self):
        print("Osiris command. Available subcommands:")
        print("  hf <arch>   - Decode hardfault registers for given architecture")

    def hfault(self, arg, from_tty):
        args = gdb.string_to_argv(arg)
        if len(args) == 0:
            print("Usage: os hf <arch>")
            print("Example: os hf cm4")
            return

        arch = args[0]
        if arch == "cm4":
            self.decode_cm4_hardfault()
        else:
            print(f"Unsupported architecture: {arch}")

    def decode_cm4_hardfault(self):
        print("\n--- HARDFAULT DECODER (Cortex-M4) ---")
        gdb.execute("set language c")

        HFSR  = gdb.parse_and_eval("*(unsigned long*)0xE000ED2C")
        CFSR  = gdb.parse_and_eval("*(unsigned long*)0xE000ED28")
        MMFAR = gdb.parse_and_eval("*(unsigned long*)0xE000ED34")
        BFAR  = gdb.parse_and_eval("*(unsigned long*)0xE000ED38")

        MMFSR = CFSR & 0xff
        BFSR  = (CFSR >> 8) & 0xff
        UFSR  = (CFSR >> 16) & 0xffff

        print("\n--- HARDFAULT REGISTERS ---")
        print(f"HFSR  = 0x{int(HFSR):08x}")
        print(f"CFSR  = 0x{int(CFSR):08x}")
        print(f"  MMFSR = 0x{int(MMFSR):02x}")
        print(f"  BFSR  = 0x{int(BFSR):02x}")
        print(f"  UFSR  = 0x{int(UFSR):04x}")
        print(f"MMFAR = 0x{int(MMFAR):08x}")
        print(f"BFAR  = 0x{int(BFAR):08x}")

        print(" HFSR reason:")
        if int(HFSR) & 0x00000002:
            print("   - VECTTBL: Bus fault on vector table read")
        if int(HFSR) & 0x40000000:
            print("   - FORCED : Escalated configurable fault (check CFSR)")

        print(" MMFSR reason (MemManage):")
        if int(MMFSR) & 0x01:
            print("   - IACCVIOL : Instruction access violation")
        if int(MMFSR) & 0x02:
            print("   - DACCVIOL : Data access violation")
        if int(MMFSR) & 0x08:
            print("   - MUNSTKERR: Unstacking error")
        if int(MMFSR) & 0x10:
            print("   - MSTKERR  : Stacking error")
        if int(MMFSR) & 0x20:
            print("   - MLSPERR  : Lazy FP state preservation error")
        if int(MMFSR) & 0x80:
            print(f"   - MMARVALID: MMFAR holds a valid fault address (0x{int(MMFAR):08x})")
        print(" BFSR reason (BusFault):")
        if int(BFSR) & 0x01:
            print("   - IBUSERR  : Instruction bus error")
        if int(BFSR) & 0x02:
            print("   - PRECISERR: Precise data bus error")
        if int(BFSR) & 0x04:
            print("   - IMPRECISERR: Imprecise data bus error")
        if int(BFSR) & 0x08:
            print("   - UNSTKERR : Unstacking error")
        if int(BFSR) & 0x10:
            print("   - STKERR   : Stacking error")
        if int(BFSR) & 0x20:
            print("   - LSPERR   : Lazy FP state preservation error")
        if int(BFSR) & 0x80:
            print(f"   - BFARVALID: BFAR holds a valid fault address (0x{int(BFAR):08x})")
        print(" UFSR reason (UsageFault):")
        if int(UFSR) & 0x0001:
            print("   - UNDEFINSTR: Undefined instruction")
        if int(UFSR) & 0x0002:
            print("   - INVSTATE  : Invalid EPSR state")
        if int(UFSR) & 0x0004:
            print("   - INVPC     : Invalid PC load (bad EXC_RETURN or BX)")
        if int(UFSR) & 0x0008:
            print("   - NOCP      : Coprocessor (FPU) access error")
        if int(UFSR) & 0x0100:
            print("   - UNALIGNED : Unaligned access")
        if int(UFSR) & 0x0200:
            print("   - DIVBYZERO : Divide-by-zero")
        print("----------------------------\n")

        gdb.execute("set language rust")

Os()
end

target remote :4242
