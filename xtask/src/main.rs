use std::{env, process::Command};
use anyhow::{bail, Context, Ok, Result};

fn host_triple() -> Result<String> {
    let out = Command::new("rustc").arg("-vV").output()
        .context("Failed to get host triple")?;

    let s = String::from_utf8(out.stdout)?;
    let triple = s.lines()
        .find_map(|l| l.strip_prefix("host: ").map(str::to_owned))
        .context("could not parse host triple")?;
    Ok(triple)
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("config") => run_config(args.collect()),
        Some(other) => bail!("unknown xtask command: {other}"),
        None => bail!("usage: xtask <command> [args]")
    }
}

fn run_config(args: Vec<String>) -> Result<()> {
    let status = Command::new("cargo")
        .args([
            "run",
            "--manifest-path", 
            "tools/config/Cargo.toml",
            "--",
        ])
        .args(&args)
        .status()
        .context("failed to run config")?;

    if !status.success() {
        bail!("config command failed with status: {status}");
    }
    Ok(())
}