use std::env;

use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        freestanding: { all(not(test), not(doctest), not(doc), not(kani), any(target_os = "none", target_os = "unknown")) },
    }

    generate_c_api();
}

fn generate_c_api() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let config: cbindgen::Config = cbindgen::Config {
        no_includes: true,
        includes: vec![
            "stdint.h".to_string(),
            "stdbool.h".to_string(),
            "stdarg.h".to_string(),
        ],
        layout: cbindgen::LayoutConfig {
            packed: Some("__attribute__((packed))".to_string()),
            ..Default::default()
        },
        language: cbindgen::Language::C,
        cpp_compat: false,
        ..Default::default()
    };

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .map_or_else(
            |error| match error {
                cbindgen::Error::ParseSyntaxError { .. } => {}
                e => panic!("{e:?}"),
            },
            |bindings| {
                bindings.write_to_file("include/bindings.h");
            },
        );
}
