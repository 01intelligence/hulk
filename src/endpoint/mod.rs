use std::fmt;

mod ellipses;
mod net;
mod setup_type;

use anyhow::ensure;
pub use ellipses::*;
pub use net::*;
use path_absolutize::Absolutize;
pub use setup_type::*;

use crate::errors;
use crate::globals::*;
use crate::strset::StringSet;

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

impl Endpoints {
    pub fn is_https(&self) -> bool {
        self.0[0].is_https()
    }

    pub fn get_string(&self, i: usize) -> String {
        self.0
            .get(i)
            .map(|e| e.to_string())
            .unwrap_or_else(|| "".to_owned())
    }

    pub fn get_all_strings(&self) -> Vec<String> {
        self.0.iter().map(|e| e.to_string()).collect()
    }

    fn check_cross_device_mounts(&self) -> anyhow::Result<()> {
        todo!()
    }
}

impl EndpointServerPools {
    pub fn get_local_pool_idx(&self, endpoint: &Endpoint) -> isize {
        for (i, p) in self.0.iter().enumerate() {
            for e in &p.endpoints.0 {
                if e.is_local && endpoint.is_local && e == endpoint {
                    return i as isize;
                }
            }
        }
        return -1;
    }

    pub fn add(&mut self, p: PoolEndpoints) -> anyhow::Result<()> {
        let mut existing = StringSet::new();
        for p in &self.0 {
            for e in &p.endpoints.0 {
                existing.add(e.to_string())
            }
        }
        for e in &p.endpoints.0 {
            ensure!(
                !existing.contains(&e.to_string()),
                "duplicate endpoints found"
            );
        }
        self.0.push(p);
        Ok(())
    }

    pub fn local_host(&self) -> String {
        for p in &self.0 {
            for e in &p.endpoints.0 {
                if e.is_local {
                    return e.url.to_string(); // TODO: right?
                }
            }
        }
        "".to_owned()
    }

    pub fn local_disk_paths(&self) -> Vec<String> {
        let mut disks = Vec::new();
        for p in &self.0 {
            for e in &p.endpoints.0 {
                if e.is_local {
                    disks.push(e.url.path().to_owned());
                }
            }
        }
        disks
    }

    pub fn first_local(&self) -> bool {
        self.0[0].endpoints.0[0].is_local
    }

    pub fn https(&self) -> bool {
        self.0[0].endpoints.is_https()
    }

    pub fn num_of_endpoints(&self) -> usize {
        self.0
            .iter()
            .map(|p| p.endpoints.0.len())
            .fold(0, |acc, c| acc + c)
    }

    pub fn host_names(&self) -> Vec<String> {
        let mut found = StringSet::new();
        for p in &self.0 {
            for e in &p.endpoints.0 {
                let host = e.url.host_str().unwrap();
                if found.contains(host) {
                    found.add(host.to_owned());
                }
            }
        }
        found.to_vec()
    }

    pub fn peers(&self) -> (Vec<String>, String) {
        let mut all = StringSet::new();
        let mut local = None;
        for p in &self.0 {
            for e in &p.endpoints.0 {
                if e.typ() != EndpointType::Url {
                    continue;
                }
                let host = e.url.host_str().unwrap().to_owned();
                let port = e.url.port_or_known_default().unwrap();
                let peer = crate::net::Host::new(host, Some(port)).to_string();
                all.add(peer.clone());
                if e.is_local {
                    if port.to_string() == *GLOBAL_PORT.lock().unwrap() {
                        local = Some(peer);
                    }
                }
            }
        }
        (all.to_vec(), local.unwrap_or_else(|| "".to_owned()))
    }

    pub fn sorted_hosts(&self) -> Vec<String> {
        let (mut peers, local_peer) = self.peers();
        peers.sort_unstable();
        peers.into_iter().filter(|h| h != &local_peer).collect()
    }
}

pub(self) async fn create_endpoints(
    server_addr: &str,
    found_local: bool,
    args: &[&[&str]],
) -> anyhow::Result<(Endpoints, SetupType)> {
    check_local_server_addr(server_addr).await?;

    let server_addr_port = split_host_port(server_addr).unwrap();

    let mut endpoints = Vec::new();
    let mut setup_type;

    // For single arg, return FS setup.
    if args.len() == 1 && args[0].len() == 1 {
        let mut endpoint = Endpoint::new(args[0][0])?;
        endpoint.update_is_local().await?;
        ensure!(
            endpoint.typ() == EndpointType::Path,
            errors::UiErrorInvalidFSEndpoint.msg("use path style endpoint for FS setup".to_owned())
        );
        endpoints.push(endpoint);
        setup_type = SetupType::Fs;
        return Ok((Endpoints(endpoints), setup_type));
    }

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
