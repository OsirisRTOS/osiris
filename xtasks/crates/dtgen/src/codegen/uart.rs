//! UART specific code generation bus and console registry.

use super::*;

#[derive(Clone, Copy)]
struct Pin {
    port: usize,
    line: u8,
    af: u8,
}

#[derive(Clone)]
struct Bus {
    node: usize,
    instance: usize,
    baud: u32,
    data_bits: u8,
    stop_bits: u8,
    parity: u8,
    flow_control: u8,
    irqn: u8,
    priority: u8,
    tx: Pin,
    rx: Pin,
    rts: Option<Pin>,
    cts: Option<Pin>,
    compatible: String,
}

/// Parse a pinctrl node name like `usart1_tx_pa9` or `lpuart1_rts_pg6`
/// into the signal role. Returns `None` for unrecognised roles
/// (don't panic — keeps DT extension cheap and avoids the SPI NSS trap).
fn parse_uart_role(name: &str) -> Option<&'static str> {
    let mut parts = name.split('_');
    let periph = parts.next()?;
    let signal = parts.next()?;
    let is_uart =
        periph.starts_with("usart") || periph.starts_with("uart") || periph.starts_with("lpuart");
    if !is_uart {
        return None;
    }
    match signal {
        "tx" => Some("tx"),
        "rx" => Some("rx"),
        "rts" => Some("rts"),
        "cts" => Some("cts"),
        _ => None,
    }
}

fn decode_pinctrl<'a>(dt: &'a DeviceTree, pinctrl: &[u32]) -> Vec<(&'static str, Pin)> {
    let mut pins = Vec::new();
    for ph in pinctrl {
        let Some(idx) = dt.resolve_phandle_idx(*ph) else {
            continue;
        };
        let pin = &dt.nodes[idx];
        let Some(role) = parse_uart_role(&pin.name) else {
            continue;
        };
        let Some(pin_ctrl_idx) = pin.parent else {
            continue;
        };
        let pin_ctrl = &dt.nodes[pin_ctrl_idx];

        let decoded = match_compatible!(&pin_ctrl.compatible, {
            "st,stm32-pinctrl" => {
                let pinmux = match pin.extra.get("pinmux") {
                    Some(PropValue::U32Array(v)) if !v.is_empty() => v[0],
                    Some(PropValue::U32(v)) => *v,
                    _ => continue,
                };
                let (port_idx, line, mode) = decode_stm32_pinmux(pinmux);
                let base = pin_ctrl
                    .reg
                    .and_then(|(base, _)| usize::try_from(base).ok())
                    .unwrap_or_else(|| {
                        panic!(
                            "Pin controller node {} is missing a valid reg base",
                            pin_ctrl.name
                        )
                    });
                Pin { port: base + (port_idx * 0x400), line, af: mode }
            }
        });
        let Some(pin) = decoded else { continue };
        pins.push((role, pin));
    }
    pins
}

fn is_uart_node(node: &Node) -> bool {
    node.compatible
        .iter()
        .any(|c| c.contains("usart") || c.contains("uart") || c.contains("lpuart"))
}

/// Node index of the `chosen.osiris,console` UART, if any.
fn console_node_idx(dt: &DeviceTree) -> Option<usize> {
    let chosen_idx = dt.by_name.get("chosen").and_then(|v| v.first()).copied()?;
    let path = match dt.nodes[chosen_idx].extra.get("osiris,console")? {
        PropValue::Str(s) => s.as_str(),
        _ => return None,
    };
    resolve_path(dt, path)
}

fn collect(dt: &DeviceTree, console_idx: Option<usize>) -> Vec<Bus> {
    let mut out: Vec<Bus> = Vec::new();
    for (idx, node) in dt.nodes.iter().enumerate() {
        if !is_enabled(node) {
            continue;
        }
        if !is_uart_node(node) {
            continue;
        }
        let Some((base, _)) = node.reg else {
            panic!("dtgen: UART node `{}` is missing a `reg` base", node.name);
        };
        let Ok(instance) = usize::try_from(base) else {
            panic!(
                "dtgen: UART node `{}` has an out-of-range `reg` base {base:#x}",
                node.name
            );
        };

        let baud = match node.extra.get("current-speed") {
            Some(PropValue::U32(v)) => *v,
            _ => 115200,
        };
        let data_bits = match node.extra.get("data-bits") {
            Some(PropValue::U32(v)) => *v as u8,
            _ => 8,
        };
        let stop_bits = match node.extra.get("stop-bits") {
            Some(PropValue::U32(v)) => *v as u8,
            _ => 1,
        };
        let parity = match node.extra.get("parity") {
            Some(PropValue::Str(s)) => match s.as_str() {
                "odd" => 1,
                "even" => 2,
                _ => 0,
            },
            Some(PropValue::U32(v)) => *v as u8,
            _ => 0,
        };
        let flow_control = match node.extra.get("hw-flow-control") {
            Some(PropValue::Empty) => 1,
            Some(PropValue::U32(v)) if *v != 0 => 1,
            _ => 0,
        };

        // IT mode needs `interrupts = <irqn priority>`; the console is
        // blocking, so keep it even without one (else: no boot console).
        let (irqn, priority) = match node.interrupts.as_slice() {
            [irqn, priority, ..] => (*irqn as u8, *priority as u8),
            _ if Some(idx) == console_idx => (0, 0),
            _ => panic!(
                "dtgen: UART node `{}` has no `interrupts` property (required for \
                 the interrupt-driven path; only the chosen console may omit it)",
                node.name
            ),
        };

        let (mut tx, mut rx, mut rts, mut cts) = (None, None, None, None);
        for (key, value) in &node.extra {
            if !key.starts_with("pinctrl-") {
                continue;
            }
            let PropValue::U32Array(pinctrl) = value else {
                continue;
            };
            for (role, pin) in decode_pinctrl(dt, pinctrl) {
                let dst = match role {
                    "tx" => &mut tx,
                    "rx" => &mut rx,
                    "rts" => &mut rts,
                    "cts" => &mut cts,
                    _ => continue,
                };
                *dst = Some(pin);
            }
        }

        let Some(tx) = tx else {
            panic!(
                "dtgen: UART node `{}` has no `tx` pin in any pinctrl-* state",
                node.name
            );
        };
        let Some(rx) = rx else {
            panic!(
                "dtgen: UART node `{}` has no `rx` pin in any pinctrl-* state",
                node.name
            );
        };
        if flow_control == 1 && (rts.is_none() || cts.is_none()) {
            panic!(
                "dtgen: UART node `{}` sets `hw-flow-control` but is missing an `rts`/`cts` pin",
                node.name
            );
        }

        let compatible = node
            .compatible
            .first()
            .cloned()
            .unwrap_or_else(|| "st,stm32-uart".to_string());

        out.push(Bus {
            node: idx,
            instance,
            baud,
            data_bits,
            stop_bits,
            parity,
            flow_control,
            irqn,
            priority,
            tx,
            rx,
            rts,
            cts,
            compatible,
        });
    }
    out
}

pub fn emit_registry(dt: &DeviceTree) -> TokenStream {
    let console_idx = console_node_idx(dt);
    let buses = collect(dt, console_idx);
    let console_const = match console_idx.and_then(|n| buses.iter().position(|b| b.node == n)) {
        Some(i) => quote! { Some(#i) },
        None => quote! { None },
    };

    let pin_tokens = |p: Pin| {
        let port = p.port;
        let line = p.line;
        let af = p.af;
        quote! { UartPin { port: #port, line: #line, af: #af } }
    };

    let opt_pin = |p: Option<Pin>| match p {
        Some(p) => {
            let pin = pin_tokens(p);
            quote! { Some(#pin) }
        }
        None => quote! { None },
    };

    let entries = buses.iter().enumerate().map(|(i, b)| {
        let index = i as u8;
        let node = b.node;
        let instance = b.instance;
        let baud = b.baud;
        let data_bits = b.data_bits;
        let stop_bits = b.stop_bits;
        let parity = b.parity;
        let flow_control = b.flow_control;
        let irqn = b.irqn;
        let priority = b.priority;
        let tx = pin_tokens(b.tx);
        let rx = pin_tokens(b.rx);
        let rts = opt_pin(b.rts);
        let cts = opt_pin(b.cts);
        let compatible = b.compatible.as_str();
        quote! {
            UartRegistryEntry {
                index: #index,
                node: #node,
                instance: #instance,
                compatible: #compatible,
                tx: #tx,
                rx: #rx,
                rts: #rts,
                cts: #cts,
                baud: #baud,
                data_bits: #data_bits,
                stop_bits: #stop_bits,
                parity: #parity,
                flow_control: #flow_control,
                irqn: #irqn,
                priority: #priority,
            },
        }
    });

    quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct UartPin {
            pub port: usize,
            pub line: u8,
            pub af: u8,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct UartRegistryEntry {
            pub index: u8,
            pub node: usize,
            pub instance: usize,
            pub compatible: &'static str,
            pub tx: UartPin,
            pub rx: UartPin,
            pub rts: Option<UartPin>,
            pub cts: Option<UartPin>,
            pub baud: u32,
            pub data_bits: u8,
            pub stop_bits: u8,
            pub parity: u8,
            pub flow_control: u8,
            pub irqn: u8,
            pub priority: u8,
        }

        pub const UART_REGISTRY: &[UartRegistryEntry] = &[
            #(#entries)*
        ];

        #[doc = "index into UART_REGISTRY of the chosen.osiris,console node, resolved at codegen time"]
        pub const CONSOLE_UART: Option<usize> = #console_const;
    }
}

pub fn emit_query_api() -> TokenStream {
    quote! {
        pub fn uart_by_index(idx: u8) -> Option<&'static UartRegistryEntry> {
            UART_REGISTRY.iter().find(|e| e.index == idx)
        }

        pub fn uart_by_compatible(compatible: &str, ord: usize) -> Option<&'static UartRegistryEntry> {
            let mut matches = 0usize;
            for e in UART_REGISTRY {
                if e.compatible == compatible {
                    if matches == ord {
                        return Some(e);
                    }
                    matches += 1;
                }
            }
            None
        }
    }
}
