use std::sync::atomic::{AtomicUsize, Ordering};

mod backtrace;
mod config;
mod drain;
mod entry;
mod logger;
mod reqinfo;

pub use self::backtrace::*;
pub use config::*;
pub use drain::*;
pub use entry::*;
pub use logger::*;
pub use reqinfo::*;
pub use slog::Level;

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(usize::MAX);

pub fn get_log_level() -> Option<Level> {
    Level::from_usize(LOG_LEVEL.load(Ordering::Relaxed))
}

pub fn set_log_level(level: Level) {
    LOG_LEVEL.store(level.as_usize(), Ordering::SeqCst);
}