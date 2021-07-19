use thiserror::Error;

#[derive(Error, Debug)]
pub enum EventError {
    #[error("invalid filter name '{0}'")]
    InvalidFilterName(String),
    #[error("more than one prefix in filter rule")]
    FilterNamePrefix,
    #[error("more than one suffix in filter rule")]
    FilterNameSuffix,
    #[error("invalid filter value '{0}'")]
    InvalidFilterValue(String),
    #[error("duplicate event name '{0}' found")]
    DuplicateEventName(String),
    #[error("topic or cloud function configuration is not supported")]
    UnsupportedConfiguration,
    #[error("duplicate queue configuration")]
    DuplicateQueueConfiguration,
    #[error("unknown region '{0}'")]
    UnknownRegion(String),
    #[error("ARN '{0}' not found")]
    ArnNotFound(String),
    #[error("invalid ARN '{0}'")]
    InvalidArn(String),
    #[error("invalid target id '{0}'")]
    InvalidTargetId(String),
    #[error("invalid event name '{0}'")]
    InvalidEventName(String),
}
