use std::sync::atomic::{AtomicUsize, Ordering};

mod audit;
mod backtrace;
mod config;
mod console;
mod drain;
mod entry;
mod logger;
mod reqinfo;
mod webhook;

pub use audit::*;
pub use config::*;
pub use console::*;
pub use drain::*;
pub use entry::*;
pub use logger::*;
pub use reqinfo::*;
pub use slog::Level;
pub use webhook::*;

pub use self::backtrace::*;

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(usize::MAX);

pub fn get_log_level() -> Option<Level> {
    Level::from_usize(LOG_LEVEL.load(Ordering::Relaxed))
}

pub fn set_log_level(level: Level) {
    LOG_LEVEL.store(level.as_usize(), Ordering::SeqCst);
}
