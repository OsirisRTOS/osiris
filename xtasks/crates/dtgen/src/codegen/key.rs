//! gpio-keys registry codegen. One entry per child of a `compatible =
//! "gpio-keys"` node. The shared (port, line, active_low, label) part
//! goes through [`super::collect_gpio_children`]; the key-specific
//! `osiris,code` / `zephyr,code`, debounce, wakeup, and polling flags
//! are read off the same child node here.
//!
//! Both `osiris,code` and `zephyr,code` are accepted; the
//! osiris-namespaced property wins when both are present.

use super::*;

/// Fallback when `osiris,irq-priority` is absent on a key node.
const DEFAULT_KEY_IRQ_PRIORITY: u8 = 5;

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
    irq_priority: u8,
}

fn collect_keys(dt: &DeviceTree) -> Vec<Key> {
    collect_gpio_children(dt, "gpio-keys")
        .into_iter()
        .map(|c| {
            // Prefer osiris-namespaced; fall back to canonical Zephyr.
            let code = match c
                .child
                .extra
                .get("osiris,code")
                .or_else(|| c.child.extra.get("zephyr,code"))
            {
                Some(PropValue::U32(v)) => *v,
                Some(PropValue::U32Array(v)) if !v.is_empty() => v[0],
                _ => 0,
            };

            let debounce_ms = match c.child.extra.get("debounce-interval-ms") {
                Some(PropValue::U32(v)) => *v,
                _ => 0,
            };

            let wakeup_source = if c.child.extra.contains_key("wakeup-source") {
                1
            } else {
                0
            };

            // `polling-mode` is accepted but ignored at runtime — we
            // always wire IRQs. Parsing it just avoids rejecting boards
            // that set it.
            let polling_mode = if c.child.extra.contains_key("polling-mode") {
                1
            } else {
                0
            };

            let irq_priority = match c.child.extra.get("osiris,irq-priority") {
                Some(PropValue::U32(v)) => u8::try_from(*v).unwrap_or_else(|_| {
                    panic!(
                        "gpio-keys child {} `osiris,irq-priority` {} out of u8 range",
                        c.child.name, v
                    )
                }),
                _ => DEFAULT_KEY_IRQ_PRIORITY,
            };

            Key {
                node: c.child_idx,
                port: c.port,
                line: c.line,
                active_low: c.active_low,
                label: c.label,
                code,
                debounce_ms,
                wakeup_source,
                polling_mode,
                irq_priority,
            }
        })
        .collect()
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
        let irq_priority = k.irq_priority;
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
                irq_priority: #irq_priority,
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
            pub irq_priority: u8,
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
