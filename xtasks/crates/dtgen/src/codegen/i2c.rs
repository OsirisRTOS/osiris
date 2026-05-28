//! I2C specific code generation for bus and device registry.

use super::*;
use quote::format_ident;

#[derive(Clone, Copy)]
struct BusPin {
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

#[derive(Clone)]
struct Bus {
    node: usize,
    instance: usize,
    hz: u32,
    timingr: u32,
    scl: BusPin,
    sda: BusPin,
}

#[derive(Clone)]
struct Dev {
    node: usize,
    bus_node: usize,
    bus_instance: usize,
    address: u16,
    enable: Option<DevPin>,
    compatible: String,
}

fn decode_stm32_pinmux(pinmux: u32) -> (usize, u8, u8) {
    let port_idx = ((pinmux >> 9) & 0x1f) as usize;
    let line = ((pinmux >> 5) & 0x0f) as u8;
    let mode = (pinmux & 0x1f) as u8;
    (port_idx, line, mode)
}

fn decode_pinctrl<'a>(dt: &'a DeviceTree, pinctrl: &[u32]) -> Vec<(&'a str, BusPin)> {
    fn parse_i2c_role(name: &str) -> Option<&'static str> {
        let mut parts = name.split('_');
        let periph = parts.next()?;
        let signal = parts.next()?;
        if !periph.starts_with("i2c") {
            return None;
        }
        match signal {
            "scl" => Some("scl"),
            "sda" => Some("sda"),
            _ => None,
        }
    }

    let mut pins = Vec::new();
    for ph in pinctrl {
        let Some(pin) = dt.resolve_phandle_idx(*ph) else {
            panic!("Invalid phandle in pinctrl: {ph:#x}");
        };
        let pin = &dt.nodes[pin];

        let Some(pin_ctrl) = pin.parent else {
            panic!("Pin node has no pin-controller?");
        };
        let pin_ctrl = &dt.nodes[pin_ctrl];

        let (port, line, mode) = match_compatible!(&pin_ctrl.compatible, {
            "st,stm32-pinctrl" => {
                let pinmux = match pin.extra.get("pinmux") {
                    Some(PropValue::U32Array(v)) if !v.is_empty() => v[0],
                    Some(PropValue::U32(v)) => *v,
                    _ => panic!("Pin node missing pinmux property"),
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
                let port = base + (port_idx * 0x400);
                (port, line, mode)
            }
        })
        .unwrap_or_else(|| panic!("Unsupported pin-controller: {:?}", pin_ctrl.compatible));

        let name = pin.name.as_str();
        let role = parse_i2c_role(name).unwrap_or_else(|| {
            panic!("Unable to determine I2C signal role from pin name: {name}");
        });

        pins.push((
            role,
            BusPin {
                port,
                pin: line,
                af: mode,
            },
        ));
    }

    pins
}

fn collect_buses(dt: &DeviceTree) -> Vec<Bus> {
    let mut buses = Vec::new();

    for (idx, node) in dt.nodes.iter().enumerate() {
        if !is_enabled(node) {
            continue;
        }
        if node.compatible.iter().all(|c| c != "osiris,stm32l4-i2c") {
            continue;
        }

        let Some((base, _)) = node.reg else {
            continue;
        };
        let Ok(instance) = usize::try_from(base) else {
            continue;
        };

        let (mut scl, mut sda) = (None, None);
        for (key, value) in &node.extra {
            if !key.starts_with("pinctrl-") {
                continue;
            }

            let PropValue::U32Array(pinctrl) = value else {
                continue;
            };

            for (name, pin) in decode_pinctrl(dt, pinctrl) {
                let dst = match name {
                    "scl" => &mut scl,
                    "sda" => &mut sda,
                    _ => continue,
                };
                *dst = Some(pin);
            }
        }

        buses.push(Bus {
            node: idx,
            instance,
            hz: match node.extra.get("clock-frequency") {
                Some(PropValue::U32(v)) => *v,
                _ => 100_000,
            },
            timingr: match node.extra.get("osiris,timingr") {
                Some(PropValue::U32(v)) => *v,
                _ => panic!("I2C bus node {} missing osiris,timingr property", node.name),
            },
            scl: scl.expect("I2C pinctrl should define scl"),
            sda: sda.expect("I2C pinctrl should define sda"),
        });
    }

    buses
}

fn collect_devices(dt: &DeviceTree, buses: &[Bus]) -> Vec<Dev> {
    let mut devices = Vec::new();

    for bus in buses {
        let bus_node = &dt.nodes[bus.node];
        for child_idx in &bus_node.children {
            let child = &dt.nodes[*child_idx];
            if !is_enabled(child) || child.compatible.is_empty() {
                continue;
            }

            let address = child
                .reg
                .and_then(|(v, _)| u16::try_from(v).ok())
                .unwrap_or_else(|| {
                    panic!(
                        "I2C device node {} has no valid reg property for address",
                        child.name
                    )
                });

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
                            "Invalid GPIO controller phandle for enable pin in I2C device node {}",
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
            let en_line = en.pin;
            let en_active_low = en.active_low;
            quote! {
                &[I2cDevPin {
                    port: #en_port,
                    line: #en_line,
                    active_low: #en_active_low,
                }]
            }
        } else {
            quote! { &[] }
        };

        quote! {
            I2cDeviceRegistryEntry {
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
        let bus_devices_ident = format_ident!("I2C_BUS_{}_DEVICES", bus_node);
        let bus_dev_entries = devices
            .iter()
            .filter(|d| d.bus_node == bus_node)
            .map(dev_entry_tokens);

        quote! {
            const #bus_devices_ident: &[I2cDeviceRegistryEntry] = &[
                #(#bus_dev_entries)*
            ];
        }
    });

    let bus_entries = buses.iter().map(|b| {
        let node = b.node;
        let instance = b.instance;
        let hz = b.hz;
        let timingr = b.timingr;
        let bus_devices_ident = format_ident!("I2C_BUS_{}_DEVICES", node);

        let scl_port = b.scl.port;
        let scl_line = b.scl.pin;
        let scl_af = b.scl.af;
        let scl = quote! {
            I2cBusPin {
                port: #scl_port,
                line: #scl_line,
                af: #scl_af,
            }
        };

        let sda_port = b.sda.port;
        let sda_line = b.sda.pin;
        let sda_af = b.sda.af;
        let sda = quote! {
            I2cBusPin {
                port: #sda_port,
                line: #sda_line,
                af: #sda_af,
            }
        };

        quote! {
            I2cBusRegistryEntry {
                node: #node,
                instance: #instance,
                hz: #hz,
                timingr: #timingr,
                scl: #scl,
                sda: #sda,
                devices: #bus_devices_ident,
            },
        }
    });

    quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct I2cBusPin {
            pub port: usize,
            pub line: u8,
            pub af: u8,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct I2cDevPin {
            pub port: usize,
            pub line: u8,
            pub active_low: u8,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct I2cDeviceRegistryEntry {
            pub node: usize,
            pub bus_node: usize,
            pub bus_instance: usize,
            pub address: u16,
            pub enable: &'static [I2cDevPin],
            pub compatible: &'static str,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct I2cBusRegistryEntry {
            pub node: usize,
            pub instance: usize,
            pub hz: u32,
            pub timingr: u32,
            pub scl: I2cBusPin,
            pub sda: I2cBusPin,
            pub devices: &'static [I2cDeviceRegistryEntry],
        }

        #(#bus_device_arrays)*

        pub const I2C_BUS_REGISTRY: &[I2cBusRegistryEntry] = &[
            #(#bus_entries)*
        ];
    }
}

pub fn emit_query_api() -> TokenStream {
    quote! {
        pub fn i2c_bus_by_dev(dev: &I2cDeviceRegistryEntry) -> Option<&'static I2cBusRegistryEntry> {
            I2C_BUS_REGISTRY.iter().find(|b| b.node == dev.bus_node)
        }

        pub fn i2c_device_by_compatible(compatible: &str, ord: usize) -> Option<&'static I2cDeviceRegistryEntry> {
            let mut matches = 0usize;
            for bus in I2C_BUS_REGISTRY {
                for dev in bus.devices {
                    if dev.compatible == compatible {
                        if matches == ord {
                            return Some(dev);
                        }
                        matches += 1;
                    }
                }
            }
            None
        }
    }
}
