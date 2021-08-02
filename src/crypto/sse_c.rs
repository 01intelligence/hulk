use std::collections::HashMap;
use std::fmt;

use actix_http::header::HeaderMap;

use super::*;

// Represents AWS SSE-C. It provides functionality to handle
// SSE-C requests.
pub const SSEC: Ssec = Ssec {};

pub struct Ssec {}

impl SseType for Ssec {
    fn is_requested(&self, headers: &HeaderMap) -> bool {
        todo!()
    }

    fn is_encrypted(&self, map: &HashMap<String, String>) -> bool {
        todo!()
    }
}

impl fmt::Display for Ssec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SSE-C")
    }
}
