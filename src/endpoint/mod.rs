mod ellipses;

use std::fmt;

pub use ellipses::*;

pub enum EndpointType {
    Path,
    Url,
}

pub struct Endpoint {
    url: url::Url,
    is_local: bool,
}

pub struct Endpoints(Vec<Endpoint>);

pub struct PoolEndpoints {
    set_count: usize,
    drives_per_set: usize,
    endpoints: Endpoints,
}

pub struct EndpointServerPools(Vec<PoolEndpoints>);

impl Endpoint {
    pub fn typ(&self) -> EndpointType {
        if !self.url.has_host() {
            EndpointType::Path
        } else {
            EndpointType::Url
        }
    }

    pub fn is_https(&self) -> bool {
        self.url.scheme() == "https"
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.url.has_host() {
            write!(f, "{}", self.url.path())
        } else {
            write!(f, "{}", self.url.to_string())
        }
    }
}
