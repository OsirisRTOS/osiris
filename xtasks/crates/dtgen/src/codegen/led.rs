//! gpio-leds registry codegen. Shared (port, line, active_low, label)
//! extraction via [`super::collect_gpio_children`]; `default-state`
//! (Linux/Zephyr binding) is parsed here into a `LedDefaultState` enum.

use super::*;

#[derive(Clone, Copy)]
enum DefaultState {
    Off,
    On,
    Keep,
}

impl DefaultState {
    fn from_node(node: &crate::ir::Node) -> Self {
        match node.extra.get("default-state") {
            Some(PropValue::Str(s)) => match s.as_str() {
                "on" => DefaultState::On,
                "keep" => DefaultState::Keep,
                "off" => DefaultState::Off,
                other => panic!(
                    "gpio-leds child {}: unknown `default-state` value {:?} \
                     (expected \"on\", \"off\", or \"keep\")",
                    node.name, other
                ),
            },
            None => DefaultState::Off,
            _ => panic!(
                "gpio-leds child {}: `default-state` must be a string",
                node.name
            ),
        }
    }

    fn tokens(self) -> TokenStream {
        match self {
            DefaultState::Off => quote! { LedDefaultState::Off },
            DefaultState::On => quote! { LedDefaultState::On },
            DefaultState::Keep => quote! { LedDefaultState::Keep },
        }
    }
}

#[derive(Clone)]
struct Led {
    node: usize,
    port: usize,
    line: u8,
    active_low: u8,
    label: String,
    default_state: DefaultState,
}

fn collect_leds(dt: &DeviceTree) -> Vec<Led> {
    let leds: Vec<Led> = collect_gpio_children(dt, "gpio-leds")
        .into_iter()
        .map(|c| Led {
            node: c.child_idx,
            port: c.port,
            line: c.line,
            active_low: c.active_low,
            label: c.label,
            default_state: DefaultState::from_node(c.child),
        })
        .collect();

    // `led_by_label` returns the first match; reject reused non-empty
    // labels so the lookup is unambiguous.
    for i in 0..leds.len() {
        if leds[i].label.is_empty() {
            continue;
        }
        for j in (i + 1)..leds.len() {
            if leds[i].label == leds[j].label {
                panic!(
                    "gpio-leds label `{}` is reused — labels must be unique when set",
                    leds[i].label
                );
            }
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
        let default_state = l.default_state.tokens();
        quote! {
            LedRegistryEntry {
                node: #node,
                port: #port,
                line: #line,
                active_low: #active_low,
                label: #label,
                default_state: #default_state,
            },
        }
    });

    quote! {
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum LedDefaultState {
            Off,
            On,
            /// Preserve current ODR (warm restart); cold boot reads analog → 0.
            Keep,
        }

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct LedRegistryEntry {
            pub node: usize,
            pub port: usize,
            pub line: u8,
            pub active_low: u8,
            pub label: &'static str,
            pub default_state: LedDefaultState,
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
