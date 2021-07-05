use std::fmt;

mod ellipses;
mod net;
mod setup_type;

use anyhow::ensure;
pub use ellipses::*;
pub use net::*;
use path_absolutize::Absolutize;
pub use setup_type::*;

use crate::globals::*;

#[derive(Debug, Eq, PartialEq)]
pub enum EndpointType {
    Path,
    Url,
}

#[derive(Debug, Eq, PartialEq)]
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
            !arg.is_empty() && arg != SLASH_SEPARATOR && arg != "\\",
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

            ensure!(url.port_or_known_default().is_some(), "invalid port");

            url.set_path(&path_clean::clean(url.path()));
            ensure!(
                !url.path().is_empty() && url.path() != SLASH_SEPARATOR && url.path() != "\\",
                "empty or root path is not supported in URL endpoint"
            );

            Ok(Endpoint {
                url,
                is_local: false,
            })
        } else {
            ensure!(
                url != Err(url::ParseError::InvalidPort),
                url::ParseError::InvalidPort
            );
            ensure!(
                url != Err(url::ParseError::EmptyHost),
                url::ParseError::EmptyHost
            );
            ensure!(
                !is_host_ip(arg),
                "invalid URL endpoint format: missing scheme http or https"
            );
            use path_absolutize::*;
            let path = std::path::Path::new(arg).absolutize()?;
            let path = path_clean::clean(
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("invalid path"))?,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_new() {
        let u2 = url::Url::parse("https://example.org/path").unwrap();
        let u4 = url::Url::parse("http://192.168.253.200/path").unwrap();
        let u12 = url::Url::parse("http://server/path").unwrap();
        let u_any = u2.clone();
        let cases = vec![
            (
                "/foo",
                Endpoint {
                    url: url::Url::from_file_path("/foo").unwrap(),
                    is_local: true,
                },
                EndpointType::Path,
                false,
            ),
            (
                "https://example.org/path",
                Endpoint {
                    url: u2,
                    is_local: false,
                },
                EndpointType::Url,
                false,
            ),
            (
                "http://192.168.253.200/path",
                Endpoint {
                    url: u4,
                    is_local: false,
                },
                EndpointType::Url,
                false,
            ),
            (
                "",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                SLASH_SEPARATOR,
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "\\",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "c://foo",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "ftp://foo",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "http://server/path?location",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "http://:/path",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "http://:8080/path",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "http://server:/path",
                Endpoint {
                    url: u12,
                    is_local: false,
                },
                EndpointType::Url,
                false,
            ),
            (
                "https://93.184.216.34:808080/path",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "http://server:8080//",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "http://server:8080/",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
            (
                "192.168.1.210:9000",
                Endpoint {
                    url: u_any.clone(),
                    is_local: false,
                },
                EndpointType::Url,
                true,
            ),
        ];
        for (i, (arg, expected_endpoint, expected_type, expected_err)) in
            cases.into_iter().enumerate()
        {
            // println!("Case {}", i);
            let endpoint = Endpoint::new(arg).map(|mut endpoint| {
                endpoint.update_is_local();
                endpoint
            });
            match endpoint {
                Err(err) => {
                    assert!(expected_err, err.to_string());
                }
                Ok(endpoint) => {
                    assert!(!expected_err, endpoint.to_string());
                    assert_eq!(endpoint.url.as_str(), expected_endpoint.url.as_str());
                    assert_eq!(endpoint, expected_endpoint);
                    assert_eq!(endpoint.typ(), expected_type);
                }
            }
        }
    }
}
