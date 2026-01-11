use std::path::{Path, PathBuf};

use config::error::Error;
use config::ui::launch_config_ui;

use clap::Parser;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Subcommand>,

    #[arg(long)]
    root: Option<PathBuf>,
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Load a preset configuration.
    Load {
        /// Name of the preset to load. Must correspond to a file in the `presets/` directory without the `.toml` extension.
        preset: String,
        /// Do not ask for confirmation.
        #[arg(long, default_value_t = false)]
        no_confirm: bool,
    },
    /// Remove all configuration options from the config file.
    Clean {
        /// Do not ask for confirmation.
        #[arg(long, default_value_t = false)]
        no_confirm: bool,
    },
}

pub fn main() {
    logging::init();
    let cli = Cli::parse();

    let current_dir = match &cli.root {
        Some(path) => path.clone(),
        None => config::error::fail_on_error(std::env::current_dir().map_err(Error::from), None),
    };

    log::info!("Current directory: {}", current_dir.display());

    match cli.cmd {
        Some(Subcommand::Load { preset, no_confirm }) => {
            config::error::fail_on_error(run_load_preset(&preset, no_confirm, &current_dir), None)
        }
        Some(Subcommand::Clean { no_confirm }) => {
            config::error::fail_on_error(run_clean(no_confirm, &current_dir), None)
        }
        None => {
            run_ui(&current_dir);
        },
    };
}

fn ask_confirmation(prompt: &str) -> bool {
    print!("{} (y/N): ", prompt);

    if let Err(_) = std::io::Write::flush(&mut std::io::stdout()) {
        return false;
    }

    let mut input = String::new();

    if let Err(_) = std::io::stdin().read_line(&mut input) {
        return false;
    }

    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

fn run_load_preset(preset_name: &str, no_confirm: bool, current_dir: &Path) -> Result<(), Error> {
    // Load the preset file from the `presets/` directory.
    let preset_path = PathBuf::from("presets").join(format!("{preset_name}.toml"));
    let preset = config::load_toml(&preset_path)?;

    let config_path = current_dir.join(".cargo/config.toml");

    let mut config = config::load_toml_mut(&config_path)?;

    // Ask for confirmation
    if !no_confirm
        && !ask_confirmation(&format!(
            "Are you sure you want to apply the preset '{preset_name}' to {}?\nThis will overwrite all existing configuration options.",
            config_path.display()
        ))
    {
        log::info!("Abort.");
        return Ok(());
    }

    config::apply_preset(&mut config, &preset)?;

    // Write back to file
    std::fs::write(&config_path, config.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;

    log::info!(
        "Applied preset '{preset_name}' to {}",
        config_path.display()
    );
    Ok(())
}

fn run_clean(no_confirm: bool, current_dir: &Path) -> Result<(), Error> {
    // Ask for confirmation
    if !no_confirm
        && !ask_confirmation(
            "Are you sure you want to remove all configuration options from .cargo/config.toml?",
        )
    {
        log::info!("Abort.");
        return Ok(());
    }

    let config_path = current_dir.join(".cargo/config.toml");

    let mut config = config::load_toml_mut(&config_path)?;

    config.retain(|key, _value| {
        if key == "alias" {
            return true;
        }

        false
    });

    // Write back to file
    std::fs::write(&config_path, config.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;

    log::info!(
        "Cleaned Osiris-related configuration options from {}",
        config_path.display()
    );
    Ok(())
}

fn run_ui(current_dir: &Path) {
    let config_path = current_dir.join(".cargo/config.toml");

    let node = config::load_config(&current_dir, "options.toml");

    let ignored = vec![];
    let state = config::load_state(&node, Some(&config_path), &ignored);

    config::error::fail_on_error(launch_config_ui(&node, state, &current_dir), None);
}
