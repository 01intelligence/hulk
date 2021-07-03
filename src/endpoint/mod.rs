use std::fmt;

mod ellipses;
mod net;
mod setup_type;

pub use ellipses::*;
pub use net::*;
pub use setup_type::*;

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

pub(self) fn create_endpoints(
    server_addr: &str,
    found_local: bool,
    args: &[&str],
) -> anyhow::Result<(Endpoints, SetupType)> {
    todo!()
}
