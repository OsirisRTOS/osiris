use spdlog::re_export::log;

mod logging;

fn main() {
    logging::init_log(log::LevelFilter::Trace);
}