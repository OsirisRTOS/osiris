//! SPI specific code generation bus and device registry.

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
    sck: BusPin,
    miso: BusPin,
    mosi: BusPin,
}

#[derive(Clone)]
struct Dev {
    node: usize,
    bus_node: usize,
    bus_instance: usize,
    cs: DevPin,
    enable: Option<DevPin>,
    max_hz: u32,
    cpol: u8,
    cpha: u8,
    bits_per_word: u8,
    cs_setup_delay_us: u32,
    cs_hold_delay_us: u32,
    cs_inactive_delay_us: u32,
    compatible: String,
}

/// Decodes the STM32_PINMUX macro encoding.
fn decode_stm32_pinmux(pinmux: u32) -> (usize, u8, u8) {
    let port_idx = ((pinmux >> 9) & 0x1f) as usize;
    let line = ((pinmux >> 5) & 0x0f) as u8;
    let mode = (pinmux & 0x1f) as u8;
    (port_idx, line, mode)
}

/// Decodes pinctrl phandles to extract port, line, and alternate function for SPI pins.
fn decode_pinctrl<'a>(dt: &'a DeviceTree, pinctrl: &[u32]) -> Vec<(&'a str, BusPin)> {
    /// Parses a node name like "spi1_sck_pa5" to extract the SPI signal role (sck, miso, mosi).
    fn parse_spi_role(name: &str) -> Option<&'static str> {
        let mut parts = name.split('_');
        let periph = parts.next()?;
        let signal = parts.next()?;
        if !periph.starts_with("spi") {
            return None;
        }
        match signal {
            "sck" => Some("sck"),
            "miso" => Some("miso"),
            "mosi" => Some("mosi"),
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
            // New pinctrl decoders go here.
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

        let role = match parse_spi_role(name) {
            Some(r) => r,
            None => {
                println!("Unable to determine SPI signal role from pin name: {name}");
                continue;
            }
        };

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

fn decode_dev_pins(dt: &DeviceTree, gpios: &[u32]) -> Vec<DevPin> {
    let pins = decode_gpio_pins(dt, gpios);
    pins.into_iter()
        .map(|(node, pin, active_low)| {
            let port = node
                .reg
                .and_then(|(base, _)| usize::try_from(base).ok())
                .unwrap_or_else(|| {
                    panic!("Invalid GPIO controller phandle for CS: {}", node.name);
                });
            DevPin {
                port,
                pin,
                active_low,
            }
        })
        .collect()
}

fn collect_buses(dt: &DeviceTree) -> Vec<Bus> {
    let mut buses: Vec<Bus> = Vec::new();

    // Look for spi controllers
    for (idx, node) in dt.nodes.iter().enumerate() {
        if !is_enabled(node) {
            continue;
        }
        if node.compatible.iter().all(|c| c != "osiris,stm32l4-spi") {
            continue;
        }

        let Some((base, _)) = node.reg else {
            continue;
        };

        let Ok(instance) = usize::try_from(base) else {
            continue;
        };

        let (mut sck, mut miso, mut mosi) = (None, None, None);

        for (key, value) in &node.extra {
            if !key.starts_with("pinctrl-") {
                continue;
            }

            let PropValue::U32Array(pinctrl) = value else {
                continue;
            };

            for (name, pin) in decode_pinctrl(dt, pinctrl) {
                let dst = match name {
                    "sck" => &mut sck,
                    "miso" => &mut miso,
                    "mosi" => &mut mosi,
                    _ => continue,
                };

                *dst = Some(pin);
            }
        }

        let sck = sck.expect("SPI pinctrl should define sck");
        let miso = miso.expect("SPI pinctrl should define miso");
        let mosi = mosi.expect("SPI pinctrl should define mosi");

        let bus = Bus {
            node: idx,
            instance,
            sck,
            miso,
            mosi,
        };
        buses.push(bus);
    }
    buses
}

fn collect_devices(dt: &DeviceTree, buses: &[Bus]) -> Vec<Dev> {
    let mut devices: Vec<Dev> = Vec::new();

    for bus in buses {
        let bus_node = &dt.nodes[bus.node];
        let cs = match bus_node.extra.get("cs-gpios") {
            Some(PropValue::U32Array(v)) => v,
            _ => panic!("SPI bus node {} missing cs-gpios property", bus_node.name),
        };
        let cs = decode_dev_pins(dt, cs);

        // The peripherals connected to the bus
        for child_idx in &bus_node.children {
            let child = &dt.nodes[*child_idx];
            if !is_enabled(child) || child.compatible.is_empty() {
                continue;
            }

            let cs_idx = child
                .reg
                .and_then(|(v, _)| usize::try_from(v).ok())
                .unwrap_or_else(|| {
                    panic!(
                        "SPI device node {} has no reg property to specify CS index",
                        child.name
                    );
                });

            let cs = cs[cs_idx];

            let max_hz = match child.extra.get("spi-max-frequency") {
                Some(PropValue::U32(v)) => *v,
                _ => panic!(
                    "SPI device node {} missing spi-max-frequency property",
                    child.name
                ),
            };

            let enable = match child.extra.get("enable-gpios") {
                Some(PropValue::U32Array(v)) if v.len() >= 3 => v.as_slice(),
                _ => &[], // Optional - device may have no enable GPIOs
            };
            // TODO: Only one enable pin for now.
            let enable = decode_dev_pins(dt, enable);
            if enable.len() > 1 {
                panic!(
                    "Multiple enable GPIOs specified for SPI device node {}, but only one is supported",
                    child.name
                );
            }
            let enable = enable.first().cloned();

            let cpol = if child.extra.contains_key("spi-cpol") {
                1
            } else {
                0
            };

            let cpha = if child.extra.contains_key("spi-cpha") {
                1
            } else {
                0
            };

            let bits_per_word = match child.extra.get("spi-word-size") {
                Some(PropValue::U32(v)) => u8::try_from(*v).unwrap_or_else(|_| {
                    panic!(
                        "SPI device node {} has spi-word-size out of u8 range: {}",
                        child.name, v
                    )
                }),
                _ => 8,
            };

            let cs_setup_delay_us = match child.extra.get("spi-cs-setup-delay-us") {
                Some(PropValue::U32(v)) => *v,
                _ => 0,
            };
            let cs_hold_delay_us = match child.extra.get("spi-cs-hold-delay-us") {
                Some(PropValue::U32(v)) => *v,
                _ => 0,
            };
            let cs_inactive_delay_us = match child.extra.get("spi-cs-inactive-delay-us") {
                Some(PropValue::U32(v)) => *v,
                _ => match child.extra.get("spi-post-delay-us") {
                    Some(PropValue::U32(v)) => *v,
                    _ => 0,
                },
            };
            devices.push(Dev {
                node: *child_idx,
                bus_node: bus.node,
                bus_instance: bus.instance,
                cs,
                enable,
                max_hz,
                cpol,
                cpha,
                bits_per_word,
                cs_setup_delay_us,
                cs_hold_delay_us,
                cs_inactive_delay_us,
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

        let cs_port = d.cs.port;
        let cs_line = d.cs.pin;
        let cs_active_low = d.cs.active_low;
        let cs = quote! {
            SpiDevPin {
                port: #cs_port,
                line: #cs_line,
                active_low: #cs_active_low,
            }
        };

        let enable = if let Some(en) = d.enable {
            let en_port = en.port;
            let en_line = en.pin;
            let en_active_low = en.active_low;
            quote! {
                &[SpiDevPin {
                    port: #en_port,
                    line: #en_line,
                    active_low: #en_active_low,
                }]
            }
        } else {
            quote! { &[] }
        };

        let max_hz = d.max_hz;
        let cpol = d.cpol;
        let cpha = d.cpha;
        let bits_per_word = d.bits_per_word;
        let cs_setup_delay_us = d.cs_setup_delay_us;
        let cs_hold_delay_us = d.cs_hold_delay_us;
        let cs_inactive_delay_us = d.cs_inactive_delay_us;
        let compatible = d.compatible.as_str();

        quote! {
            SpiDeviceRegistryEntry {
                node: #node,
                bus_node: #bus_node,
                bus_instance: #bus_instance,
                cs: #cs,
                enable: #enable,
                max_hz: #max_hz,
                cpol: #cpol,
                cpha: #cpha,
                bits_per_word: #bits_per_word,
                cs_setup_delay_us: #cs_setup_delay_us,
                cs_hold_delay_us: #cs_hold_delay_us,
                cs_inactive_delay_us: #cs_inactive_delay_us,
                compatible: #compatible,
            },
        }
    };

    let bus_device_arrays = buses.iter().map(|b| {
        let bus_node = b.node;
        let bus_devices_ident = format_ident!("SPI_BUS_{}_DEVICES", bus_node);
        let bus_dev_entries = devices
            .iter()
            .filter(|d| d.bus_node == bus_node)
            .map(dev_entry_tokens);

        quote! {
            const #bus_devices_ident: &[SpiDeviceRegistryEntry] = &[
                #(#bus_dev_entries)*
            ];
        }
    });

    let bus_entries = buses.iter().map(|b| {
        let node = b.node;
        let instance = b.instance;
        let bus_devices_ident = format_ident!("SPI_BUS_{}_DEVICES", node);

        let sck_port = b.sck.port;
        let sck_line = b.sck.pin;
        let sck_af = b.sck.af;
        let sck = quote! {
            SpiBusPin {
                port: #sck_port,
                line: #sck_line,
                af: #sck_af,
            }
        };

        let miso_port = b.miso.port;
        let miso_line = b.miso.pin;
        let miso_af = b.miso.af;
        let miso = quote! {
            SpiBusPin {
                port: #miso_port,
                line: #miso_line,
                af: #miso_af,
            }
        };

        let mosi_port = b.mosi.port;
        let mosi_line = b.mosi.pin;
        let mosi_af = b.mosi.af;
        let mosi = quote! {
            SpiBusPin {
                port: #mosi_port,
                line: #mosi_line,
                af: #mosi_af,
            }
        };

        quote! {
            SpiBusRegistryEntry {
                node: #node,
                instance: #instance,
                sck: #sck,
                miso: #miso,
                mosi: #mosi,
                devices: #bus_devices_ident,
            },
        }
    });

    quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct SpiBusPin {
            pub port: usize,
            pub line: u8,
            pub af: u8,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct SpiDevPin {
            pub port: usize,
            pub line: u8,
            pub active_low: u8,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct SpiDeviceRegistryEntry {
            pub node: usize,
            pub bus_node: usize,
            pub bus_instance: usize,
            pub cs: SpiDevPin,
            pub enable: &'static [SpiDevPin],
            pub max_hz: u32,
            pub cpol: u8,
            pub cpha: u8,
            pub bits_per_word: u8,
            pub cs_setup_delay_us: u32,
            pub cs_hold_delay_us: u32,
            pub cs_inactive_delay_us: u32,
            pub compatible: &'static str,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct SpiBusRegistryEntry {
            pub node: usize,
            pub instance: usize,
            pub sck: SpiBusPin,
            pub miso: SpiBusPin,
            pub mosi: SpiBusPin,
            pub devices: &'static [SpiDeviceRegistryEntry],
        }

        #(#bus_device_arrays)*

        pub const SPI_BUS_REGISTRY: &[SpiBusRegistryEntry] = &[
            #(#bus_entries)*
        ];
    }
}

pub fn emit_query_api() -> TokenStream {
    quote! {
        pub fn spi_bus_by_dev(dev: &SpiDeviceRegistryEntry) -> Option<&'static SpiBusRegistryEntry> {
            SPI_BUS_REGISTRY.iter().find(|b| b.node == dev.bus_node)
        }

        pub fn spi_device_by_compatible(compatible: &str, ord: usize) -> Option<&'static SpiDeviceRegistryEntry> {
            let mut matches = 0usize;
            for bus in SPI_BUS_REGISTRY {
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
