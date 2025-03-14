//! Logging module.
//!
//! Provides a wrapper around log and env_logger.

#![warn(missing_docs)]


use std::env;
use std::io::Write;

use env_logger::{Builder, Env};
use log::Level;


/// Initializes the global logger.
/// 
/// Accepts an optional minimum logging level. If None is passed, the logger will default 
/// to the value contained in the RUST_LOG environment variable. If RUST_LOG is not set,
/// the logger will default to log `info` and above.
/// 
/// # Example
/// ```
/// logging::init(Level::Warn);
/// 
/// trace!("helloworld!"); // no output.
/// debug!("helloworld!"); // no output.
/// info!("helloworld!");  // no output.
/// warn!("helloworld!");
/// error!("helloworld!");
/// ```
pub fn init(level: Option<Level>) {
    if let Some(l) = level { 
        set_logging_level_before_builder_init(l);
    }

    Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] {}.",
                record.level(),
                record.args()
            )
        })
        .init();
}


/// Sets the RUST_LOG environment variable.
/// 
/// To see a change, this function must be called before the `env_logger::Builder` is initialized.
fn set_logging_level_before_builder_init(level: Level) {
    unsafe {
        env::set_var("RUST_LOG", level.as_str());
    }
}


/// Converts a u8 integer to a logging level.
/// 
/// Defaults to `trace` if the conversion fails.
pub fn u8_to_level(value: u8) -> Level {
    return match value {
        1 => log::Level::Debug,
        2 => log::Level::Info,
        3 => log::Level::Warn,
        4 => log::Level::Error,
        _ => log::Level::Trace,
    }
}
