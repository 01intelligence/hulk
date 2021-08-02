use std::collections::HashMap;
use std::fmt;

use actix_web::http::HeaderMap;

pub trait SseType: fmt::Display {
    fn is_requested(&self, headers: &HeaderMap) -> bool;
    fn is_encrypted(&self, map: &HashMap<String, String>) -> bool;
}
