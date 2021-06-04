mod entry;
mod logger;
mod reqinfo;
mod backtrace;
mod drain;

use std::sync::atomic::{AtomicUsize, Ordering};

pub use slog::Level;

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(usize::MAX);

pub fn get_log_level() -> Option<Level> {
    Level::from_usize(LOG_LEVEL.load(Ordering::Relaxed))
}

pub fn set_log_level(level: Level) {
    LOG_LEVEL.store(level.as_usize(), Ordering::SeqCst);
}
