use anyhow::{Context, Ok, Result, bail};
use std::{path::PathBuf, process::Command};

use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Subcommand,
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    Config {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Inject symbols into the given target binary.
    InjectSyms {
        /// Target triple. If not specified, the host triple will be used.
        #[arg(long)]
        target: Option<String>,
        #[arg(long, default_value_t = false)]
        release: bool,
        #[arg(long, default_value_t = String::from("Kernel"))]
        binary: String,
    },
}

fn host_triple() -> Result<String> {
    let out = Command::new("rustc")
        .arg("-vV")
        .output()
        .context("Failed to get host triple")?;

    let s = String::from_utf8(out.stdout)?;
    let triple = s
        .lines()
        .find_map(|l| l.strip_prefix("host: ").map(str::to_owned))
        .context("could not parse host triple")?;
    Ok(triple)
}

fn init_tracing() {
    let filter = EnvFilter::try_from_env("XTASK_LOG")
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let event_fmt = tracing_subscriber::fmt::format()
        .compact()
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time();

    tracing_subscriber::fmt()
        .event_format(event_fmt)
        .with_env_filter(filter)
        .init();
}

fn main() {
    init_tracing();

    let cli = Cli::parse();

    if let Err(err) = match cli.cmd {
        Subcommand::Config { args } => run_config(args),
        Subcommand::InjectSyms {
            target,
            release,
            binary,
        } => run_inject_syms(target, release, binary),
    } {
        tracing::error!("{:?}", err);
        std::process::exit(1);
    }
}

fn run_inject_syms(target: Option<String>, release: bool, binary: String) -> Result<()> {
    let target = match target {
        Some(t) => t,
        None => host_triple()?,
    };

    // find the target directory
    let mut binary_file = PathBuf::from("target").join(&target);
    let build_type = if release { "release" } else { "debug" };
    binary_file = binary_file.join(build_type).join(&binary);

    // Check if the target directory exists
    if !binary_file.exists() {
        bail!("Target binary does not exist: {}", binary_file.display());
    }

    tracing::info!("injecting symbols into binary: {}", binary_file.display());

    let status = Command::new("python3")
        .arg("tools/injector/inject_syms.py")
        .arg("--file")
        .arg(&binary_file)
        .status()
        .context("failed to run inject_syms")?;

    if !status.success() {
        bail!("inject_syms command failed with status: {status}");
    }

    Ok(())
}

fn run_config(args: Vec<String>) -> Result<()> {
    let status = Command::new("cargo")
        .args(["run", "--manifest-path", "tools/config/Cargo.toml", "--"])
        .args(&args)
        .status()
        .context("failed to run config")?;

    if !status.success() {
        bail!("config command failed with status: {status}");
    }
    Ok(())
}
