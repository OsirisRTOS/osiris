

use clap::Parser;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Subcommand>,
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Creates a flashable image from a kernel binary 
    Assemble {

    }
}


fn main() {
    println!("Hello, world!");
}
