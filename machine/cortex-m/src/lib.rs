#![cfg_attr(all(not(disabled), cortex_m), no_std)]

cfg_select! {
    disabled => { /* The whole crate is disabled */ }
    cortex_m => {
        pub mod native;
        pub use native as hal;
    }
    _ => {
        pub mod stub;
        pub use stub as hal;
    }
}
