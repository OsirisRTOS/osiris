use std::io::Write;
use std::sync::OnceLock;

use env_logger::fmt::style::{self, Style};
use log::Level;
use style::AnsiColor;

static INIT: OnceLock<()> = OnceLock::new();

pub fn init() {
    INIT.get_or_init(|| {
        let mut builder =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

        builder.format(|buf, record| match record.level() {
            Level::Error => {
                let lvl_style = Style::new()
                    .fg_color(Some(AnsiColor::BrightRed.into()))
                    .bold();

                let msg_style = Style::new().bold();

                writeln!(
                    buf,
                    "{lvl_style}error{lvl_style:#}: {msg_style}{}{msg_style:#}",
                    record.args()
                )
            }
            Level::Warn => {
                let lvl_style = Style::new()
                    .fg_color(Some(AnsiColor::BrightYellow.into()))
                    .bold();

                writeln!(buf, "{lvl_style}warning{lvl_style:#}: {}", record.args())
            }
            Level::Info => {
                let lvl_style = Style::new()
                    .fg_color(Some(AnsiColor::BrightGreen.into()))
                    .bold();

                writeln!(buf, "{lvl_style}info{lvl_style:#}: {}", record.args())
            }
            Level::Debug => {
                let lvl_style = Style::new()
                    .fg_color(Some(AnsiColor::BrightBlue.into()))
                    .bold();
                writeln!(buf, "{lvl_style}debug{lvl_style:#}: {}", record.args())
            }
            Level::Trace => {
                let lvl_style = Style::new()
                    .fg_color(Some(AnsiColor::BrightMagenta.into()))
                    .bold();
                writeln!(buf, "{lvl_style}trace{lvl_style:#}: {}", record.args())
            }
        });

        builder.try_init().ok();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{error, info, warn};

    #[test]
    fn test_logging() {
        init();

        info!("This is an info message.");
        warn!("This is a warning message.");
        error!("This is an error message.");
    }
}
