use std::path::PathBuf;

use config::ui::launch_config_ui;


fn get_examples_dir() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    current_dir.join("examples")
}

pub fn main() {
    // Initialize logging
    config::logging::init_log(log::LevelFilter::Trace);

    let current_dir = get_examples_dir();
    println!("Current directory: {}", current_dir.display());

    let options_path = current_dir.join("assets");

    let node = config::resolve::resolve_config(&options_path)
        .expect("Failed to resolve configuration");

    //println!("Parsed configuration: {node:#?}");

    launch_config_ui(node)
        .expect("Failed to launch configuration UI");
}