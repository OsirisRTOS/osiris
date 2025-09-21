use spdlog::formatter::{PatternFormatter, pattern};

pub fn init_log(level: log::LevelFilter) {
    spdlog::init_log_crate_proxy()
        .expect("Cannot initialize log crate proxy twice. THIS IS A BUG!");
    log::set_max_level(level);

    let formatter = Box::new(PatternFormatter::new(pattern!(
        "{^{level}}: {payload}{eol}"
    )));

    for sink in spdlog::default_logger().sinks() {
        sink.set_formatter(formatter.clone());
    }
}
