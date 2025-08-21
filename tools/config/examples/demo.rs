use std::path::PathBuf;

use config::ui::launch_config_ui;

fn get_examples_dir() -> config::error::Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    Ok(current_dir.join("examples"))
}

pub fn main() {
    config::logging::init_log(log::LevelFilter::Trace);

    let current_dir = config::error::fail_on_error(get_examples_dir(), None);
    log::info!("Current directory: {}", current_dir.display());

    let options_path = current_dir.join("assets");
    let config_path = current_dir.join(".cargo/config.toml");

    let node = config::load_config(&options_path);
    let state = config::load_state(&node, Some(&config_path));

    //println!("Parsed configuration: {node:#?}");

    config::error::fail_on_error(launch_config_ui(&node, state, &current_dir), None);
}
