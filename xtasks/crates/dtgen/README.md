# dtgen 

`dtgen` parses a Device Tree Source (`.dts`) file and emits a `dt.rs` file containing a complete static representation of the device tree. This file is included via `include!` and provides a query API usable both at compile time (proc macros) and at runtime. 

---

## Including the generated file

In your crate:

```rust
include!(concat!(env!("OUT_DIR"), "/dt.rs"));
```

Or if dtgen was invoked with a custom output path:

```rust
include!("path/to/dt.rs");
```

---

## Types

### `PropValue`

Represents a raw DTS property value.

```rust
pub enum PropValue {
    Empty,                                  // boolean flag property, e.g. gpio-controller
    U32(u32),                               // single cell, e.g. current-speed = <115200>
    U32Array(&'static [u32]),               // cell array, e.g. clocks = <&rcc 1 0x4000>
    Str(&'static str),                      // string, e.g. status = "okay"
    StringList(&'static [&'static str]),    // string list, e.g. compatible = "a", "b"
    Bytes(&'static [u8]),                   // raw byte array
}
```

### `Peripheral`

Every node with at least one `compatible` string is emitted as a `Peripheral`.

```rust
pub struct Peripheral {
    pub node:       usize,                                  // index into NODES[]
    pub compatible: &'static [&'static str],                // all compatible strings
    pub reg:        Option<(usize, usize)>,                 // (base_addr, size)
    pub interrupts: &'static [u32],                         // interrupt numbers
    pub phandle:    Option<u32>,                            // phandle value if present
    pub props:      &'static [(&'static str, PropValue)],   // all extra properties
}
```

### `TreeNode`

Topology-only node - every node in the tree including structural ones.

```rust
pub struct TreeNode {
    pub name:     &'static str,
    pub phandle:  Option<u32>,
    pub parent:   Option<usize>,
    pub children: &'static [usize],
}
```

---

## Static arrays

```rust
NODES: &[TreeNode]          // every node in the tree, depth-first order
PERIPHERALS: &[Peripheral]  // every node that has a compatible string
MODEL: &str                 // /model property or first root compatible
STDOUT: Option<&str>        // first compatible of the /chosen stdout-path target
```

---

## Peripheral methods

### Compatible matching

```rust
// exact match against any compatible string
p.is_compatible("st,stm32-uart")

// substring match - useful for class-level matching
p.compatible_contains("uart")
```

### Property access

```rust
// raw PropValue
p.prop("current-speed")                  // Option<PropValue>

// typed convenience accessors
p.prop_u32("current-speed")              // Option<u32>
p.prop_str("status")                     // Option<&'static str>
p.prop_u32_array("clocks")               // Option<&'static [u32]>
```

### Register / interrupt access

```rust
p.reg_base()    // Option<usize>    base address
p.reg_size()    // Option<usize>    mapped size
p.interrupts    // &[u32]           all interrupt numbers
```

### Phandle resolution

Phandle arrays are stored as raw `U32Array` props.
The first element of each phandle entry is the phandle value of the provider node.

```rust
// resolve a phandle to its Peripheral
if let Some(PropValue::U32Array(cells)) = p.prop("clocks") {
    let clock_phandle = cells[0];
    if let Some(clock) = p.resolve_phandle(clock_phandle) {
        let freq = clock.prop_u32("clock-frequency");
    }
}
```

### Status / enabled

```rust
// returns true if status is absent or "okay"
// returns false if status = "disabled"
p.is_enabled()
```

### Tree navigation

```rust
p.tree_node()                   // &'static TreeNode
p.tree_node().parent_node()     // Option<&'static TreeNode>
p.tree_node().iter_children()   // impl Iterator<Item = (usize, &'static TreeNode)>
```

---

---
## Free query functions

### By compatible string
```rust
// first enabled match
peripheral_by_compatible("st,stm32-uart")   // Option<&'static Peripheral>

// all enabled matches - e.g. multiple UARTs
peripherals_by_compatible("st,stm32-uart")  // impl Iterator<Item = &'static Peripheral>
```
### By phandle
```rust
peripheral_by_phandle(1)    // Option<&'static Peripheral>
```
### By node index
```rust
peripheral_by_node(7)   // Option<&'static Peripheral>
```
### By name
Node names are only unique among siblings. Use the scoped or path forms when the
name may appear under multiple parents (e.g. `channel@0` under multiple ADC nodes).
```rust
// all matches across the entire tree - name with or without unit address
peripherals_by_name("channel")             // impl Iterator<Item = &'static Peripheral>
peripherals_by_name("channel@0")           // exact unit-address match also works

// scoped to a specific parent - safe when names are not globally unique
peripheral_by_name_under("channel", adc1.node)  // Option<&'static Peripheral>

// unambiguous full path - with or without unit addresses at each segment
peripheral_by_path("soc/adc@50040000/channel@17")   // Option<&'static Peripheral>
peripheral_by_path("soc/adc/channel")               // first match at each level
```
---

## `chosen` submodule
```rust
// O(1) direct index into PERIPHERALS, resolved at codegen time from /chosen stdout-path
chosen::stdout()         // Option<&'static Peripheral>

// raw constants - all are Option<_>, None if absent from /chosen
chosen::STDOUT           // Option<usize>  — index into PERIPHERALS
chosen::BOOTARGS         // Option<&str>
chosen::INITRD_START     // Option<u32>
chosen::INITRD_END       // Option<u32>
```
---
## `aliases` submodule
```rust
// resolve an alias name to its Peripheral
aliases::resolve("serial1")   // Option<&'static Peripheral>

// raw table if you need to iterate
aliases::ALIASES              // &[(&str, usize)]  — (alias_name, node_index)
```
---
## `memory` submodule
```rust
// all declared memory regions, sorted by base address
// entries are (node_name, base_address, size_in_bytes)
memory::REGIONS                             // &[(&str, usize, usize)]
memory::region_by_name("memory@20000000")   // Option<(usize, usize)> — (base, size)
memory::total_bytes()                       // usize
```
---
## Common query patterns

### Find the console UART

```rust
let console = chosen::stdout()
    .expect("no stdout-path in /chosen");
let base = console.reg_base().expect("console has no reg");
let baud = console.prop_u32("current-speed").unwrap_or(115200);
```

### Find all enabled UARTs
```rust
for uart in peripherals_by_compatible("st,stm32-uart") {
    let base = uart.reg_base().unwrap();
    let irq  = uart.interrupts.first().copied();
}
```

### Resolve a clock dependency
```rust
let uart = peripheral_by_compatible("st,stm32-uart").unwrap();
if let Some(PropValue::U32Array(cells)) = uart.prop("clocks") {
    // cells = [phandle, ...clock specifier cells...]
    let rcc = peripheral_by_phandle(cells[0]).expect("clock provider not found");
    let freq = rcc.prop_u32("clock-frequency").unwrap_or(0);
}
```

### Find a GPIO controller by phandle
```rust
// DTS: led-gpios = <&gpioa 5 0>
// emitted as: PropValue::U32Array(&[gpioa_phandle, 5, 0])
if let Some(PropValue::U32Array(cells)) = node.prop("led-gpios") {
    let gpio  = peripheral_by_phandle(cells[0]).unwrap();
    let pin   = cells[1];
    let flags = cells[2];
}
```

### Walk children of a node
```rust
if let Some(leds) = peripherals_by_name("leds").next() {
    for (child_idx, child_node) in leds.tree_node().iter_children() {
        if let Some(child_periph) = peripheral_by_node(child_idx) {
            // process each LED child peripheral
        }
    }
}
```

### Find an ADC channel by path
```rust
// unambiguous even though "channel@17" appears under multiple ADC nodes
let vbat = peripheral_by_path("soc/adc@50040000/channel@17")
    .expect("VBAT channel not found");
```

### Filter by compatible then check a prop
```rust
let spi = peripherals_by_compatible("st,stm32-spi")
    .find(|p| p.prop_u32("clock-frequency") == Some(1_000_000));
```

### Set up memory regions
```rust
for (name, base, size) in memory::REGIONS {
    mpu_configure_region(base, size);
}
```

---
## CLI invocation
```
dtgen <input.dts> <output.rs> [-I <include_dir>...]
```
```bash
dtgen board.dts src/dt.rs
dtgen board.dts out/dt.rs -I vendor/stm32/include -I vendor/cmsis/include
```
## `build.rs` integration
```rust
fn main() {
    let dts = std::path::Path::new("board.dts");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap())
        .join("dt.rs");
    dtgen::run(dts, &[], &out).expect("dtgen failed");
    println!("cargo:rerun-if-changed=board.dts");
}
```
