use lazy_static::lazy_static;
use slog::{Drain, Duplicate};

lazy_static! {
    static ref GLOBAL_LOG_GUARD: (slog_scope::GlobalLoggerGuard, ()) = {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, slog::slog_o!());

        let scope_guard = slog_scope::set_global_logger(logger);
        let log_guard = slog_stdlog::init().unwrap();
        (scope_guard, log_guard)
    };
}

pub struct MultipleDrain<D1: Drain, D2: Drain> {
    pub drains: Duplicate<D1, D2>,
}

impl<D1: Drain, D2: Drain> MultipleDrain<D1, D2> {
    pub fn new(drain1: D1, drain2: D2) -> Self {
        MultipleDrain {
            drains: Duplicate(drain1, drain2),
        }
    }
}
