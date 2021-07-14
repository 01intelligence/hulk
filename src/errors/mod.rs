mod typed_errors;
mod ui_errors;

pub use typed_errors::*;
pub use ui_errors::*;

pub trait AsError {
    fn as_error<E: std::error::Error + 'static>(&self) -> Option<&E>;
}

impl AsError for anyhow::Error {
    fn as_error<E: std::error::Error + 'static>(&self) -> Option<&E> {
        for cause in self.chain() {
            if let Some(err) = cause.downcast_ref::<E>() {
                return Some(err);
            }
        }
        None
    }
}
