use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApiError {
    #[error("Operation timed out")]
    OperationTimedOut,
}
