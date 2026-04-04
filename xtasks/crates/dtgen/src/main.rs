#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg(not(target_os = "none"))]

use clap::Parser;
use std::path::PathBuf;

// dtgen CLI — thin wrapper over lib::run
//
// Usage:
//   dtgen <input.dts> <output.rs> [-I <include_dir>...]
//
// Examples:
//   dtgen board.dts out/device.rs
//   dtgen board.dts out/device.rs -I vendor/stm32/include -I vendor/cmsis/include

#[derive(Parser)]
#[command(name = "dtgen", version, about)]
struct Args {
    input: PathBuf,  // input .dts file
    output: PathBuf, // output .rs file

    #[arg(short = 'I', value_name = "DIR")]
    include_dirs: Vec<PathBuf>, // extra include directories, forwarded to cpp preprocessor
}

fn main() {
    logging::init();
    let args = Args::parse();
    let refs: Vec<&std::path::Path> = args.include_dirs.iter().map(|p| p.as_path()).collect();

    let dt = dtgen::parse_dts(&args.input, &refs).unwrap_or_else(|e| {
        log::error!("dtgen error: Failed to parse device tree: {e}");
        std::process::exit(1);
    });

    let output = args.output.as_path();
    std::fs::create_dir_all(output.parent().unwrap()).unwrap_or_else(|e| {
        log::error!("dtgen error: Failed to create output directory: {e}");
        std::process::exit(1);
    });

    let content = dtgen::generate_rust(&dt);

    std::fs::write(&args.output, content).unwrap_or_else(|e| {
        log::error!("dtgen error: Failed to write output file: {e}");
        std::process::exit(1);
    });
}
