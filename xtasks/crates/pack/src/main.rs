use std::path::PathBuf;

use clap::Parser;

mod bootinfo;
mod elf;
mod image;
mod pack;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Specifies the init binary to use in the image.
    /// This can either be a path to a cargo project root or a path to an ELF binary.
    #[arg(short, long , default_value_t = String::from("."))]
    init: String,

    /// Specifies the kernel to use in the image.
    /// This can either be a path to a cargo project root or a path to an ELF binary.
    #[arg(short, long, default_value_t = String::from("."))]
    kernel: String,

    /// Overrides the target triple to use when searching for binaries in cargo projects.
    /// If not specified, the target triple will be read from the cargo configuration if available.
    #[arg(long)]
    target: Option<String>,

    /// The output path of the created image.
    #[arg(short, long, default_value_t = String::from("osiris.img"))]
    output: String,

    /// Whether to use the release profile when searching for binaries in cargo projects.
    #[arg(long, default_value_t = false)]
    release: bool,
}

fn fail_on_error<T, E: std::fmt::Display>(res: Result<T, E>) -> T {
    match res {
        Ok(v) => v,
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    }
}

fn main() {
    logging::init();

    let cli = Cli::parse();

    let profile = if cli.release { "release" } else { "debug" };

    let init_info = fail_on_error(pack::resolve_binary(&cli.init, &cli.target, profile));

    let kernel_info = fail_on_error(pack::resolve_binary(&cli.kernel, &cli.target, profile));

    let output = PathBuf::from(cli.output);
    fail_on_error(pack::pack(&init_info, &mut kernel_info.clone(), &output));
}
