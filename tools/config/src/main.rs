use config::error::Error;
use config::ui::launch_config_ui;

pub fn main() {
    config::logging::init_log(log::LevelFilter::Trace);

    let current_dir =
        config::error::fail_on_error(std::env::current_dir().map_err(Error::from), None);
    log::info!("Current directory: {}", current_dir.display());

    let config_path = current_dir.join(".cargo/config.toml");

    let node = config::load_config(&current_dir, "options.toml");
    let state = config::load_state(&node, Some(&config_path));

    config::error::fail_on_error(launch_config_ui(&node, state, &current_dir), None);
}
