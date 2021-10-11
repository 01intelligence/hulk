use actix_web::http::{HeaderMap, StatusCode};
use derivative::Derivative;

use crate::utils;
use crate::utils::Duration;

#[derive(Clone, Derivative)]
#[derivative(Default)]
pub enum TraceType {
    #[derivative(Default)]
    Http,
    Os,
    Storage,
}

#[derive(Clone, Derivative)]
#[derivative(Default)]
pub struct TraceInfo {
    pub trace_type: TraceType,

    pub node_name: String,
    pub fn_name: String,
    #[derivative(Default(value = "utils::now()"))]
    pub time: utils::DateTime,

    pub req_info: Option<TraceRequestInfo>,
    pub resp_info: Option<TraceResponseInfo>,
    pub call_stats: Option<TraceCallStats>,

    pub storage_stats: Option<TraceStorageStats>,
    pub os_stats: Option<TraceOsStats>,
}

#[derive(Clone)]
pub struct TraceRequestInfo {
    pub time: utils::DateTime,
    pub proto: String,
    pub method: String,
    pub path: String,
    pub raw_query: String,
    pub headers: HeaderMap,
    pub body: Option<bytes::Bytes>,
    pub client: String,
}

#[derive(Clone)]
pub struct TraceResponseInfo {
    pub time: utils::DateTime,
    pub headers: Option<HeaderMap>,
    pub body: Option<bytes::Bytes>,
    pub status_code: StatusCode,
}

#[derive(Clone, Default)]
pub struct TraceCallStats {
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub latency: Duration,
    pub time_to_first_byte: Duration,
}

#[derive(Clone)]
pub struct TraceStorageStats {
    pub path: String,
    pub duration: Duration,
}

#[derive(Clone)]
pub struct TraceOsStats {
    pub path: String,
    pub duration: Duration,
}
