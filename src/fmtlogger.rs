use std::sync::OnceLock;

use log::{Level, LevelFilter, Log, Metadata, Record};

const CLICOLOR_FORCE: &str = "CLICOLOR_FORCE";

/// Implements [`Log`] and a set of simple builder methods for configuration.
struct Logger {
    /// Global logging level when using this type
    level: LevelFilter,
}

impl Default for Logger {
    fn default() -> Self {
        Logger {
            level: LevelFilter::Trace,
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        &metadata.level().to_level_filter() <= &self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let target = if record.target().is_empty() {
                record.module_path().unwrap_or_default()
            } else {
                record.target()
            };

            const BLUE: &str = "\x1B[34m";
            const RED: &str = "\x1B[31m";
            const YELLOW: &str = "\x1B[33m";
            const WHITE: &str = "\x1B[37m";
            const RESET: &str = "\x1B[0m";

            let color = match record.level() {
                Level::Error => RED,
                Level::Warn => YELLOW,
                Level::Info => WHITE,
                Level::Debug => BLUE,
                Level::Trace => "",
            };

            let supports_color =
                atty::is(atty::Stream::Stderr) || std::env::var(CLICOLOR_FORCE).is_ok();
            let mut log = if let Some((file, line)) = record.file().zip(record.line()) {
                format!(
                    "{file}:{line} [{}][{target}]: {}",
                    record.level(),
                    record.args(),
                )
            } else {
                format!("[{}][{target}]: {}", record.level(), record.args())
            };
            if supports_color {
                log = format!("{}{}{}", color, log, RESET);
            }
            eprintln!("{}", log);
        }
    }

    fn flush(&self) {}
}

pub fn default() {
    static LOGGER: OnceLock<Logger> = OnceLock::new();
    let logger = LOGGER.get_or_init(|| Logger {
        level: std::env::var_os("RUST_LOG")
            .map(|x| x.into_string().ok())
            .flatten()
            .map(|x| match x.as_str() {
                "warn" => log::LevelFilter::Warn,
                "trace" => log::LevelFilter::Trace,
                "error" => log::LevelFilter::Error,
                "info" => log::LevelFilter::Info,
                "debug" => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            })
            .unwrap_or(log::LevelFilter::Trace),
    });
    log::set_max_level(logger.level);
    if let Err(err) = log::set_boxed_logger(Box::new(logger)) {
        // used const to allow for static lifetime
        eprintln!("attaching logger failed! shouldn't be possible: {:?}", err);
    }
}
