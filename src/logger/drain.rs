use std::sync::Mutex;

use lazy_static::lazy_static;
use slog::{Drain, OwnedKVList, Record};

lazy_static! {
    pub static ref LOG_TARGETS: Vec<Target> = Default::default();
    pub static ref AUDIT_TARGETS: Vec<Target> = Default::default();
    pub static ref LOG_DRAIN: Mutex<Option<MultipleDrain>> = Default::default();
    pub static ref AUDIT_DRAIN: Mutex<Option<MultipleDrain>> = Default::default();
    pub static ref LOG_LOGGER: slog::Logger = {
        let drain = LOG_DRAIN.lock().unwrap().take().unwrap();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, slog::slog_o!())
    };
    pub static ref AUDIT_LOGGER: slog::Logger = {
        let drain = AUDIT_DRAIN.lock().unwrap().take().unwrap();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, slog::slog_o!())
    };
    static ref GLOBAL_LOG_GUARD: (slog_scope::GlobalLoggerGuard, ()) = {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator)
            .use_file_location()
            .build()
            .fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, slog::slog_o!());

        let scope_guard = slog_scope::set_global_logger(logger);
        let log_guard = slog_stdlog::init().unwrap();
        (scope_guard, log_guard)
    };
}

pub struct Target {
    pub name: String,
    pub endpoint: Option<String>,
}

#[derive(Default)]
pub struct MultipleDrain {
    pub drains: Vec<Box<dyn Drain<Ok = (), Err = slog::Never> + Send>>,
}

impl MultipleDrain {
    pub fn add<D: Drain<Ok = (), Err = slog::Never> + Send + 'static>(&mut self, drain: D) {
        self.drains.push(Box::new(drain));
    }
}

impl slog::Drain for MultipleDrain {
    type Ok = ();
    type Err = slog::Never;

    fn log(&self, record: &Record, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        for drain in &self.drains {
            let _ = drain.log(record, values);
        }
        Ok(())
    }
}
