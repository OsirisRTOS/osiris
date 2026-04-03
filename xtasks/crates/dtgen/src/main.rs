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
    let args = Args::parse();
    let refs: Vec<&std::path::Path> = args.include_dirs.iter().map(|p| p.as_path()).collect();

    dtgen::run(&args.input, &refs, &args.output).unwrap_or_else(|e| {
        eprintln!("dtgen error: {e}");
        std::process::exit(1);
    });
}
