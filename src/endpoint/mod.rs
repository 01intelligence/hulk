use std::fmt;

mod ellipses;
mod net;
mod setup_type;

use anyhow::ensure;
pub use ellipses::*;
pub use net::*;
pub use setup_type::*;

use crate::globals::*;

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
    pub fn new(arg: &str) -> anyhow::Result<Endpoint> {
        // TODO: more checks
        ensure!(
            !arg.is_empty() && arg != SLASH_SEPARATOR,
            "empty or root endpoint is not supported"
        );

        let url = url::Url::parse(arg);
        if url.is_ok() && url.as_ref().unwrap().has_host() {
            let mut url = url.unwrap();
            ensure!(
                (url.scheme() == "http" || url.scheme() == "https")
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none(),
                "invalid URL endpoint format"
            );

            url.set_path(&path_clean::clean(url.path()));
            ensure!(
                !url.path().is_empty() && url.path() != SLASH_SEPARATOR,
                "empty or root path is not supported in URL endpoint"
            );

            Ok(Endpoint {
                url,
                is_local: false,
            })
        } else {
            ensure!(
                !is_host_ip(arg),
                "invalid URL endpoint format: missing scheme http or https"
            );
            let path = std::fs::canonicalize(arg)?;
            let path = path_clean::clean(
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("invalid UTF-8 path"))?,
            );
            Ok(Endpoint {
                url: url::Url::from_file_path(path).map_err(|_| anyhow::anyhow!("invalid path"))?,
                is_local: true,
            })
        }
    }

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

    pub async fn update_is_local(&mut self) -> anyhow::Result<()> {
        if !self.is_local {
            self.is_local = is_local_host(
                self.url.host_str().unwrap(),
                &self.url.port().map(|p| p.to_string()).unwrap_or_default(),
                &*GLOBAL_PORT.lock().unwrap(),
            )
            .await?;
        }
        Ok(())
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

pub(self) async fn create_endpoints(
    server_addr: &str,
    found_local: bool,
    args: &[&str],
) -> anyhow::Result<(Endpoints, SetupType)> {
    check_local_server_addr(server_addr).await?;

    let server_addr_port = split_host_port(server_addr).unwrap();

    // For single arg, return FS setup.
    if args.len() == 1 && args[0].len() == 1 {}

    todo!()
}
