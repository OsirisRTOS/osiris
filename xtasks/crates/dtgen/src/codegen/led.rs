//! gpio-leds registry codegen. One entry per child of a `compatible =
//! "gpio-leds"` node; the (port, line, active_low, label) extraction is
//! shared with gpio-keys via [`super::collect_gpio_children`].

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
            Some(PropValue::Empty) | None => DefaultState::Off,
            Some(other) => panic!(
                "gpio-leds child {}: `default-state` must be a string \
                 (\"on\" / \"off\" / \"keep\"), got {:?}",
                node.name, other
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

#[derive(Clone, Copy)]
enum OutputMode {
    PushPull,
    OpenDrain,
}

impl OutputMode {
    fn from_node(node: &crate::ir::Node) -> Self {
        match node.extra.get("osiris,output-mode") {
            Some(PropValue::Str(s)) => match s.as_str() {
                "push-pull" => OutputMode::PushPull,
                "open-drain" => OutputMode::OpenDrain,
                other => panic!(
                    "gpio-leds child {}: unknown `osiris,output-mode` value {:?} \
                     (expected \"push-pull\" or \"open-drain\")",
                    node.name, other
                ),
            },
            // Flag-form / absent → push-pull (the pre-existing default).
            Some(PropValue::Empty) | None => OutputMode::PushPull,
            Some(other) => panic!(
                "gpio-leds child {}: `osiris,output-mode` must be a string \
                 (\"push-pull\" / \"open-drain\"), got {:?}",
                node.name, other
            ),
        }
    }

    fn tokens(self) -> TokenStream {
        match self {
            OutputMode::PushPull => quote! { LedOutputMode::PushPull },
            OutputMode::OpenDrain => quote! { LedOutputMode::OpenDrain },
        }
    }
}

/// Linux/Zephyr `bias-*` triple (empty flag props). Absent → no pull.
#[derive(Clone, Copy)]
enum Pull {
    None,
    Up,
    Down,
}

impl Pull {
    fn from_node(node: &crate::ir::Node) -> Self {
        let up = node.extra.contains_key("bias-pull-up");
        let down = node.extra.contains_key("bias-pull-down");
        let disable = node.extra.contains_key("bias-disable");
        let count = up as u8 + down as u8 + disable as u8;
        if count > 1 {
            panic!(
                "gpio-leds child {}: only one of `bias-pull-up`, `bias-pull-down`, \
                 `bias-disable` may be set",
                node.name
            );
        }
        if up {
            Pull::Up
        } else if down {
            Pull::Down
        } else {
            Pull::None
        }
    }

    fn tokens(self) -> TokenStream {
        match self {
            Pull::None => quote! { LedPull::None },
            Pull::Up => quote! { LedPull::Up },
            Pull::Down => quote! { LedPull::Down },
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
    output_mode: OutputMode,
    pull: Pull,
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
            output_mode: OutputMode::from_node(c.child),
            pull: Pull::from_node(c.child),
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
        let output_mode = l.output_mode.tokens();
        let pull = l.pull.tokens();
        quote! {
            LedRegistryEntry {
                node: #node,
                port: #port,
                line: #line,
                active_low: #active_low,
                label: #label,
                default_state: #default_state,
                output_mode: #output_mode,
                pull: #pull,
            },
        }
    });

    quote! {
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum LedDefaultState {
            Off,
            On,
            /// Preserve current ODR (warm restart); cold boot reads ODR → 0.
            Keep,
        }

        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum LedOutputMode {
            PushPull,
            OpenDrain,
        }

        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum LedPull {
            None,
            Up,
            Down,
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
            pub output_mode: LedOutputMode,
            pub pull: LedPull,
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
