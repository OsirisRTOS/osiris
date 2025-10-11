use std::path::PathBuf;

use clap::Parser;

mod assemble;
mod bootinfo;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Subcommand,
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Creates a flashable image from a kernel binary
    Assemble {
        /// Target triple. If specified the tool will also search through target folders for the kernel binary.
        #[arg(long)]
        target: Option<String>,
        /// The kernel binary to bundle, when a folder is specified the tool will search for a Kernel ELF in the folder.
        /// If a folder is specified and a target triple is given, the tool will also search in the target folder.
        #[arg(short, long, default_value_t = String::from("."))]
        kernel: String,
        /// The init application binary to bundle, when a folder is specified the tool will search for a App ELF in the folder.
        /// If a folder is specified and a target triple is given, the tool will also search in the target folder.
        #[arg(short, long, default_value_t = String::from("."))]
        app: String,
        /// The flashable output image.
        #[arg(short, long, default_value_t = String::from("Osiris.img"))]
        output: String,
        release: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.cmd {
        Subcommand::Assemble { target, kernel, app, output, release } => {
            let output = PathBuf::from(output);
            assemble::assemble(target, &PathBuf::from(kernel), &PathBuf::from(app), &output, *release);
        }
    }
}
