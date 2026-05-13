//! gpio-leds registry codegen. One entry per child of a node whose
//! `compatible` is `gpio-leds`. Each entry captures the (port, line,
//! active_low) decoded from the child's `gpios` cell array via the
//! shared `decode_gpio_pins` helper, plus the optional `label`.
//!
//! Mirrors Zephyr's `gpio-leds.yaml` binding so a Zephyr board file
//! drops in unmodified.

use super::*;

#[derive(Clone)]
struct Led {
    node: usize,
    port: usize,
    line: u8,
    active_low: u8,
    label: String,
}

fn collect_leds(dt: &DeviceTree) -> Vec<Led> {
    let mut leds = Vec::new();

    for (_parent_idx, parent) in dt.nodes.iter().enumerate() {
        if !is_enabled(parent) {
            continue;
        }
        if parent.compatible.iter().all(|c| c != "gpio-leds") {
            continue;
        }

        for &child_idx in &parent.children {
            let child = &dt.nodes[child_idx];
            if !is_enabled(child) {
                continue;
            }

            let gpios = match child.extra.get("gpios") {
                Some(PropValue::U32Array(v)) => v.as_slice(),
                _ => panic!(
                    "gpio-leds child {} missing required `gpios` property",
                    child.name
                ),
            };

            let pins = decode_gpio_pins(dt, gpios);
            if pins.len() != 1 {
                panic!(
                    "gpio-leds child {} must specify exactly one GPIO ({} found)",
                    child.name,
                    pins.len()
                );
            }
            let (ctrl, line, active_low) = pins[0];
            let port = ctrl
                .reg
                .and_then(|(base, _)| usize::try_from(base).ok())
                .unwrap_or_else(|| {
                    panic!(
                        "gpio-leds child {} references controller {} with no valid reg base",
                        child.name, ctrl.name,
                    )
                });

            let label = match child.extra.get("label") {
                Some(PropValue::Str(s)) => s.clone(),
                _ => String::new(),
            };

            leds.push(Led {
                node: child_idx,
                port,
                line,
                active_low,
                label,
            });
        }
    }

    leds
}

pub fn emit_registry(dt: &DeviceTree) -> TokenStream {
    let leds = collect_leds(dt);

    let entries = leds.iter().map(|l| {
        let node = l.node;
        let port = l.port;
        let line = l.line;
        let active_low = l.active_low;
        let label = l.label.as_str();
        quote! {
            LedRegistryEntry {
                node: #node,
                port: #port,
                line: #line,
                active_low: #active_low,
                label: #label,
            },
        }
    });

    quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct LedRegistryEntry {
            pub node: usize,
            pub port: usize,
            pub line: u8,
            pub active_low: u8,
            pub label: &'static str,
        }

        pub const LED_REGISTRY: &[LedRegistryEntry] = &[
            #(#entries)*
        ];
    }
}

pub fn emit_query_api() -> TokenStream {
    quote! {
        #[doc = "resolve a /aliases entry to its LedRegistryEntry"]
        pub fn led_by_alias(name: &str) -> Option<&'static LedRegistryEntry> {
            let node = aliases::ALIASES
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, idx)| *idx)?;
            LED_REGISTRY.iter().find(|e| e.node == node)
        }

        #[doc = "find an LED by its `label` property (exact match)"]
        pub fn led_by_label(label: &str) -> Option<&'static LedRegistryEntry> {
            LED_REGISTRY.iter().find(|e| e.label == label)
        }

        #[doc = "find an LED by its NODES index"]
        pub fn led_by_node(node: usize) -> Option<&'static LedRegistryEntry> {
            LED_REGISTRY.iter().find(|e| e.node == node)
        }
    }
}
