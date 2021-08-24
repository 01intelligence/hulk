mod api_errors;
mod encryption;
mod reducible_errors;
mod storage_errors;
mod typed_errors;
mod ui_errors;

use std::error::Error;

pub use api_errors::*;
pub use encryption::*;
pub use reducible_errors::*;
pub use storage_errors::*;
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

impl AsError for std::io::Error {
    fn as_error<E: std::error::Error + 'static>(&self) -> Option<&E> {
        if let Some(err) = self.get_ref() {
            if let Some(err) = err.as_error::<E>() {
                return Some(err);
            }
        }
        None
    }
}

impl AsError for dyn std::error::Error + 'static {
    fn as_error<E: std::error::Error + 'static>(&self) -> Option<&E> {
        for cause in self.chain() {
            if let Some(err) = cause.downcast_ref::<E>() {
                return Some(err);
            }
        }
        None
    }
}

impl AsError for dyn std::error::Error + Send + Sync + 'static {
    fn as_error<E: std::error::Error + 'static>(&self) -> Option<&E> {
        (self as &(dyn std::error::Error + 'static)).as_error::<E>()
    }
}
