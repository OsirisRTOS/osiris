//! gpio-keys registry codegen. One entry per child of a `compatible =
//! "gpio-keys"` node. Captures the GPIO line, polarity, label, input
//! event code, and optional debounce / wakeup-source / polling-mode
//! flags. Mirrors Zephyr's `gpio-keys.yaml` binding.
//!
//! Both `osiris,code` and `zephyr,code` are accepted; the osiris-namespaced
//! property wins when both are present. Same pattern as the CAN codegen
//! accepting both `osiris,stm32l4-can` and `st,stm32-bxcan`.

use super::*;

#[derive(Clone)]
struct Key {
    node: usize,
    port: usize,
    line: u8,
    active_low: u8,
    label: String,
    code: u32,
    debounce_ms: u32,
    wakeup_source: u8,
    polling_mode: u8,
}

fn collect_keys(dt: &DeviceTree) -> Vec<Key> {
    let mut keys = Vec::new();

    for (_parent_idx, parent) in dt.nodes.iter().enumerate() {
        if !is_enabled(parent) {
            continue;
        }
        if parent.compatible.iter().all(|c| c != "gpio-keys") {
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
                    "gpio-keys child {} missing required `gpios` property",
                    child.name
                ),
            };

            let pins = decode_gpio_pins(dt, gpios);
            if pins.len() != 1 {
                panic!(
                    "gpio-keys child {} must specify exactly one GPIO ({} found)",
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
                        "gpio-keys child {} references controller {} with no valid reg base",
                        child.name, ctrl.name,
                    )
                });

            let label = match child.extra.get("label") {
                Some(PropValue::Str(s)) => s.clone(),
                _ => String::new(),
            };

            // Prefer osiris-namespaced; fall back to canonical Zephyr.
            let code = match child
                .extra
                .get("osiris,code")
                .or_else(|| child.extra.get("zephyr,code"))
            {
                Some(PropValue::U32(v)) => *v,
                Some(PropValue::U32Array(v)) if !v.is_empty() => v[0],
                _ => 0,
            };

            let debounce_ms = match child.extra.get("debounce-interval-ms") {
                Some(PropValue::U32(v)) => *v,
                _ => 0,
            };

            let wakeup_source = if child.extra.contains_key("wakeup-source") {
                1
            } else {
                0
            };

            // `polling-mode` is accepted but ignored at runtime — we
            // always wire IRQs. Parsing it just avoids rejecting boards
            // that set it.
            let polling_mode = if child.extra.contains_key("polling-mode") {
                1
            } else {
                0
            };

            keys.push(Key {
                node: child_idx,
                port,
                line,
                active_low,
                label,
                code,
                debounce_ms,
                wakeup_source,
                polling_mode,
            });
        }
    }

    keys
}

pub fn emit_registry(dt: &DeviceTree) -> TokenStream {
    let keys = collect_keys(dt);

    let entries = keys.iter().map(|k| {
        let node = k.node;
        let port = k.port;
        let line = k.line;
        let active_low = k.active_low;
        let label = k.label.as_str();
        let code = k.code;
        let debounce_ms = k.debounce_ms;
        let wakeup_source = k.wakeup_source;
        let polling_mode = k.polling_mode;
        quote! {
            KeyRegistryEntry {
                node: #node,
                port: #port,
                line: #line,
                active_low: #active_low,
                label: #label,
                code: #code,
                debounce_ms: #debounce_ms,
                wakeup_source: #wakeup_source,
                polling_mode: #polling_mode,
            },
        }
    });

    quote! {
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        pub struct KeyRegistryEntry {
            pub node: usize,
            pub port: usize,
            pub line: u8,
            pub active_low: u8,
            pub label: &'static str,
            pub code: u32,
            pub debounce_ms: u32,
            pub wakeup_source: u8,
            pub polling_mode: u8,
        }

        pub const KEY_REGISTRY: &[KeyRegistryEntry] = &[
            #(#entries)*
        ];
    }
}

pub fn emit_query_api() -> TokenStream {
    quote! {
        #[doc = "resolve a /aliases entry to its KeyRegistryEntry"]
        pub fn key_by_alias(name: &str) -> Option<&'static KeyRegistryEntry> {
            let node = aliases::ALIASES
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, idx)| *idx)?;
            KEY_REGISTRY.iter().find(|e| e.node == node)
        }

        #[doc = "find a key by its `label` property (exact match)"]
        pub fn key_by_label(label: &str) -> Option<&'static KeyRegistryEntry> {
            KEY_REGISTRY.iter().find(|e| e.label == label)
        }

        #[doc = "find a key by its `osiris,code` / `zephyr,code` value"]
        pub fn key_by_code(code: u32) -> Option<&'static KeyRegistryEntry> {
            KEY_REGISTRY.iter().find(|e| e.code == code)
        }
    }
}
