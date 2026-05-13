//! gpio-leds registry codegen. One entry per child of a `compatible =
//! "gpio-leds"` node; the (port, line, active_low, label) extraction is
//! shared with gpio-keys via [`super::collect_gpio_children`].

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
    collect_gpio_children(dt, "gpio-leds")
        .into_iter()
        .map(|c| Led {
            node: c.child_idx,
            port: c.port,
            line: c.line,
            active_low: c.active_low,
            label: c.label,
        })
        .collect()
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
