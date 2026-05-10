//! CAN specific code generation for bus registry.

use super::*;
use quote::format_ident;

#[derive(Clone, Copy)]
struct Pin {
    port: usize,
    pin: u8,
    af: u8,
}

#[derive(Clone, Copy)]
struct DevPin {
    port: usize,
    pin: u8,
    active_low: u8,
}

#[derive(Clone, Copy)]
struct Irq {
    irqn: u8,
    priority: u8,
}

#[derive(Clone)]
struct Bus {
    node: usize,
    instance: usize,
    bitrate_hz: u32,
    rx: Pin,
    tx: Pin,
    rx0_irq: Irq,
    rx1_irq: Irq,
    index: u8,
    tx_open_drain: u8,
}

#[derive(Clone)]
struct Dev {
    node: usize,
    bus_node: usize,
    bus_instance: usize,
    address: usize,
    enable: Option<DevPin>,
    compatible: String,
}

/// Decodes CAN pinctrl phandles, returning (role, pin) pairs where
/// role is "rx" or "tx".
fn decode_pinctrl<'a>(dt: &'a DeviceTree, pinctrl: &[u32]) -> Vec<(&'a str, Pin)> {
    /// Parses a node name like "can1_rx_pa11" to extract the CAN signal role (rx, tx).
    fn parse_can_role(name: &str) -> Option<&'static str> {
        let mut parts = name.split('_');
        let periph = parts.next()?;
        let signal = parts.next()?;
        if !periph.starts_with("can") {
            return None;
        }
        match signal {
            "rx" => Some("rx"),
            "tx" => Some("tx"),
            _ => None,
        }
    }

    let mut pins = Vec::new();
    for ph in pinctrl {
        let Some(idx) = dt.resolve_phandle_idx(*ph) else {
            panic!("Invalid phandle in pinctrl: {ph:#x}");
        };
        let pin_node = &dt.nodes[idx];

        let Some(pin_ctrl_idx) = pin_node.parent else {
            panic!("Pin node has no pin-controller?");
        };
        let pin_ctrl = &dt.nodes[pin_ctrl_idx];

        let (port, line, mode) = match_compatible!(&pin_ctrl.compatible, {
            "st,stm32-pinctrl" => {
                let pinmux = match pin_node.extra.get("pinmux") {
                    Some(PropValue::U32Array(v)) if !v.is_empty() => v[0],
                    Some(PropValue::U32(v)) => *v,
                    _ => panic!("Pin node missing pinmux property"),
                };

                let port_idx = ((pinmux >> 9) & 0x1f) as usize;
                let line = ((pinmux >> 5) & 0x0f) as u8;
                let mode = (pinmux & 0x1f) as u8;
                let base = pin_ctrl
                    .reg
                    .and_then(|(base, _)| usize::try_from(base).ok())
                    .unwrap_or_else(|| {
                        panic!(
                            "Pin controller node {} is missing a valid reg base",
                            pin_ctrl.name
                        )
                    });
                let port = base + (port_idx * 0x400);
                (port, line, mode)
            }
        })
        .unwrap_or_else(|| panic!("Unsupported pin-controller: {:?}", pin_ctrl.compatible));

        let role = match parse_can_role(pin_node.name.as_str()) {
            Some(r) => r,
            None => {
                println!(
                    "Unable to determine CAN signal role from pin name: {}",
                    pin_node.name
                );
                continue;
            }
        };

        pins.push((
            role,
            Pin {
                port,
                pin: line,
                af: mode,
            },
        ));
    }

    pins
}

/// Pulls the (irqn, priority) pair at `idx` (0=TX, 1=RX0, 2=RX1, 3=SCE)
/// from the flat `interrupts` cell array.
fn extract_irq(interrupts: &[u32], idx: usize, label: &str) -> Irq {
    let off = idx * 2;
    if interrupts.len() < off + 2 {
        panic!("CAN node missing {label} interrupt cell pair");
    }
    Irq {
        irqn: u8::try_from(interrupts[off])
            .unwrap_or_else(|_| panic!("CAN {label} irqn {} out of u8 range", interrupts[off])),
        priority: u8::try_from(interrupts[off + 1]).unwrap_or_else(|_| {
            panic!(
                "CAN {label} priority {} out of u8 range",
                interrupts[off + 1]
            )
        }),
    }
}

fn collect_buses(dt: &DeviceTree) -> Vec<Bus> {
    let mut buses: Vec<Bus> = Vec::new();
    let mut next_index: u8 = 0;

    for (idx, node) in dt.nodes.iter().enumerate() {
        if !is_enabled(node) {
            continue;
        }
        
        if node
            .compatible
            .iter()
            .all(|c| c != "osiris,stm32-bxcan")
        {
            continue;
        }

        let Some((base, _)) = node.reg else { continue };
        let Ok(instance) = usize::try_from(base) else {
            continue;
        };

        let (mut rx, mut tx) = (None, None);
        for (key, value) in &node.extra {
            if !key.starts_with("pinctrl-") {
                continue;
            }
            let PropValue::U32Array(pinctrl) = value else {
                continue;
            };
            for (name, pin) in decode_pinctrl(dt, pinctrl) {
                let dst = match name {
                    "rx" => &mut rx,
                    "tx" => &mut tx,
                    _ => continue,
                };
                *dst = Some(pin);
            }
        }
        let rx = rx.expect("CAN pinctrl should define rx");
        let tx = tx.expect("CAN pinctrl should define tx");

        // Accept either `bus-speed` (canonical Zephyr) or `bitrate`
        // (used by some pre-existing board overlays).
        let bitrate_hz = match node
            .extra
            .get("bus-speed")
            .or_else(|| node.extra.get("bitrate"))
        {
            Some(PropValue::U32(v)) => *v,
            _ => panic!(
                "CAN node {} missing bus-speed (or bitrate) property",
                node.name
            ),
        };

        let tx_open_drain = if node.extra.contains_key("drive-open-drain") {
            1
        } else {
            0
        };

        let rx0_irq = extract_irq(&node.interrupts, 1, "RX0");
        let rx1_irq = extract_irq(&node.interrupts, 2, "RX1");

        let bus = Bus {
            node: idx,
            instance,
            bitrate_hz,
            rx,
            tx,
            rx0_irq,
            rx1_irq,
            index: next_index,
            tx_open_drain,
        };
        next_index += 1;
        buses.push(bus);
    }
    buses
}

fn collect_devices(dt: &DeviceTree, buses: &[Bus]) -> Vec<Dev> {
    let mut devices = Vec::new();

    for bus in buses {
        let bus_node = &dt.nodes[bus.node];

        for child_idx in bus_node.children.iter() {
            let child = &dt.nodes[*child_idx];
            if !is_enabled(child) {
                continue;
            }

            let address = match child.reg.and_then(|(base, _)| usize::try_from(base).ok()) {
                Some(addr) => addr,
                None => continue,
            };

            let enable = match child.extra.get("enable-gpios") {
                Some(PropValue::U32Array(v)) if v.len() >= 3 => v.as_slice(),
                _ => &[],
            };
            let enable = super::decode_gpio_pins(dt, enable);
            if enable.len() > 1 {
                panic!(
                    "Multiple enable GPIOs specified for I2C device node {}, but only one is supported",
                    child.name
                );
            }
            let enable = enable.first().map(|(node, pin, active_low)| {
                let port = node
                    .reg
                    .and_then(|(base, _)| usize::try_from(base).ok())
                    .unwrap_or_else(|| {
                        panic!(
                            "Invalid GPIO controller phandle for enable pin in CAN device node {}",
                            child.name
                        );
                    });
                DevPin {
                    port,
                    pin: *pin,
                    active_low: *active_low,
                }
            });

            devices.push(Dev {
                node: *child_idx,
                bus_node: bus.node,
                bus_instance: bus.instance,
                address,
                enable,
                compatible: child.compatible[0].clone(),
            });
        }
    }
    devices
}

pub fn emit_registry(dt: &DeviceTree) -> TokenStream {
    let buses = collect_buses(dt);
    let devices = collect_devices(dt, &buses);

    let dev_entry_tokens = |d: &Dev| {
        let node = d.node;
        let bus_node = d.bus_node;
        let bus_instance = d.bus_instance;
        let address = d.address;
        let compatible = d.compatible.as_str();

        let enable = if let Some(en) = d.enable {
            let en_port = en.port;
            let en_pin = en.pin;
            let en_active_low = en.active_low != 0;
            quote! {
                &[CanDevPin {
                    port: #en_port,
                    pin: #en_pin,
                    active_low: #en_active_low,
                }]
            }
        } else {
            quote! { &[] }
        };

        quote! {
            CanDeviceRegistryEntry {
                node: #node,
                bus_node: #bus_node,
                bus_instance: #bus_instance,
                address: #address,
                enable: #enable,
                compatible: #compatible,
            },
        }
    };

    let bus_device_arrays = buses.iter().map(|b| {
        let bus_node = b.node;
        let bus_devices_ident = format_ident!("CAN_BUS_{}_DEVICES", bus_node);
        let bus_dev_entries = devices
            .iter()
            .filter(|d| d.bus_node == bus_node)
            .map(dev_entry_tokens);

        quote! {
            const #bus_devices_ident: &[CanDeviceRegistryEntry] = &[
                #(#bus_dev_entries)*
            ];
        }
    });

    let bus_entries = buses.iter().map(|b| {
        let node = b.node;
        let instance = b.instance;
        let bitrate_hz = b.bitrate_hz;
        let index = b.index;
        let tx_open_drain = b.tx_open_drain;
        let bus_devices_ident = format_ident!("CAN_BUS_{}_DEVICES", node);

        let rx_port = b.rx.port;
        let rx_line = b.rx.pin;
        let rx_af = b.rx.af;
        let rx = quote! { CanPin { port: #rx_port, line: #rx_line, af: #rx_af } };

        let tx_port = b.tx.port;
        let tx_line = b.tx.pin;
        let tx_af = b.tx.af;
        let tx = quote! { CanPin { port: #tx_port, line: #tx_line, af: #tx_af } };

        let rx0_irqn = b.rx0_irq.irqn;
        let rx0_priority = b.rx0_irq.priority;
        let rx0_irq = quote! { CanIrq { irqn: #rx0_irqn, priority: #rx0_priority } };

        let rx1_irqn = b.rx1_irq.irqn;
        let rx1_priority = b.rx1_irq.priority;
        let rx1_irq = quote! { CanIrq { irqn: #rx1_irqn, priority: #rx1_priority } };

        quote! {
            CanBusRegistryEntry {
                node: #node,
                instance: #instance,
                bitrate_hz: #bitrate_hz,
                rx: #rx,
                tx: #tx,
                rx0_irq: #rx0_irq,
                rx1_irq: #rx1_irq,
                index: #index,
                tx_open_drain: #tx_open_drain,
                devices: #bus_devices_ident,
            },
        }
    });

    quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct CanPin {
            pub port: usize,
            pub line: u8,
            pub af: u8,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct CanIrq {
            pub irqn: u8,
            pub priority: u8,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct CanDevPin {
            pub port: usize,
            pub pin: u8,
            pub active_low: bool,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct CanDeviceRegistryEntry {
            pub node: usize,
            pub bus_node: usize,
            pub bus_instance: usize,
            pub address: usize,
            pub enable: &'static [CanDevPin],
            pub compatible: &'static str,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct CanBusRegistryEntry {
            pub node: usize,
            pub instance: usize,
            pub bitrate_hz: u32,
            pub rx: CanPin,
            pub tx: CanPin,
            pub rx0_irq: CanIrq,
            pub rx1_irq: CanIrq,
            pub index: u8,
            pub tx_open_drain: u8,
            pub devices: &'static [CanDeviceRegistryEntry],
        }

        #(#bus_device_arrays)*

        pub const CAN_BUS_REGISTRY: &[CanBusRegistryEntry] = &[
            #(#bus_entries)*
        ];
    }
}

pub fn emit_query_api() -> TokenStream {
    quote! {
        pub fn can_bus_by_dev(dev: &CanDeviceRegistryEntry) -> Option<&'static CanBusRegistryEntry> {
            CAN_BUS_REGISTRY.iter().find(|b| b.node == dev.bus_node)
        }

        pub fn can_device_by_compatible(compatible: &str, ord: usize) -> Option<&'static CanBusRegistryEntry> {
            let mut matches = 0usize;
            for bus in CAN_BUS_REGISTRY {
                for dev in bus.devices {
                    if dev.compatible == compatible {
                        if matches == ord {
                            return Some(bus);
                        }
                        matches += 1;
                    }
                }
            }
            None
        }
    }
}
