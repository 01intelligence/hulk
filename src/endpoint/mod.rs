use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;

use anyhow::ensure;
use path_absolutize::Absolutize;

use crate::errors::UiError;
use crate::globals::*;
use crate::strset::StringSet;
use crate::utils::{Path, PathBuf};

mod ellipses;
mod net;
mod setup_type;

use std::net::IpAddr;

pub use ellipses::*;
pub use net::*;
pub use setup_type::*;

use crate::utils::PathExt;

#[derive(Debug, Eq, PartialEq)]
pub enum EndpointType {
    Path,
    Url,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Endpoint {
    pub url: url::Url,
    pub is_local: bool,
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
            let path = Path::new(arg).absolutize()?;
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

    pub fn host(&self) -> &str {
        self.url.host_str().unwrap_or_default()
    }

    pub async fn update_is_local(&mut self) -> anyhow::Result<()> {
        if !self.is_local {
            self.is_local = is_local_host(
                self.url.host_str().unwrap(),
                &self.url.port().map(|p| p.to_string()).unwrap_or_default(),
                &*GLOBALS.port.guard(),
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
    pub fn new(args: &[&str]) -> anyhow::Result<Endpoints> {
        let mut endpoint_type = None;
        let mut scheme = None;
        let mut unique_args = StringSet::new();
        let mut endpoints = Vec::new();
        for (i, &arg) in args.iter().enumerate() {
            let endpoint = Endpoint::new(arg)?;
            if i == 0 {
                endpoint_type = Some(endpoint.typ());
                scheme = Some(endpoint.url.scheme().to_owned());
            } else {
                ensure!(
                    Some(endpoint.typ()) == endpoint_type,
                    "mixed style endpoints are not supported"
                );
                ensure!(
                    endpoint.url.scheme() == scheme.as_ref().unwrap(),
                    "mixed scheme is not supported"
                );
            }
            let arg = endpoint.to_string();
            ensure!(!unique_args.contains(&arg), "duplicate endpoints found");
            unique_args.add(arg);
            endpoints.push(endpoint);
        }
        Ok(Endpoints(endpoints))
    }

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

    async fn check_cross_device_mounts(&self) -> anyhow::Result<()> {
        let mut abs_paths = Vec::<PathBuf>::new();
        for e in &self.0 {
            if e.is_local {
                let path: PathBuf = e
                    .url
                    .to_file_path()
                    .map_err(|_| anyhow::anyhow!("invalid file path"))?
                    .try_into()?;
                abs_paths.push(crate::fs::canonicalize(path).await?.try_into()?)
            }
        }
        crate::mount::check_cross_device(&abs_paths)
    }

    pub fn update_is_local(&mut self, found_prev_local: bool) -> anyhow::Result<()> {
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
                    if port.to_string() == *GLOBALS.port.guard() {
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

    pub fn get_local_peer(&self, host: &str, port: &str) -> String {
        let mut peers = StringSet::new();
        for ep in &self.0 {
            for endpoint in &ep.endpoints.0 {
                if endpoint.typ() != EndpointType::Url {
                    continue;
                }
                if endpoint.is_local && endpoint.url.host_str().is_some() {
                    peers.add(endpoint.url.host_str().unwrap().to_owned());
                }
            }
        }
        if peers.is_empty() {
            return join_host_port(if !host.is_empty() { host } else { "127.0.0.1" }, port);
        }
        peers.as_slice()[0].to_owned()
    }
}

pub(self) async fn create_endpoints(
    server_addr: &str,
    found_local: bool,
    args_list: &[&[&str]],
) -> anyhow::Result<(Endpoints, SetupType)> {
    check_local_server_addr(server_addr).await?;

    let (_, server_addr_port) = split_host_port(server_addr).unwrap();

    let mut endpoints = Vec::new();

    // For single arg, return FS setup.
    if args_list.len() == 1 && args_list[0].len() == 1 {
        let mut endpoint = Endpoint::new(args_list[0][0])?;
        endpoint.update_is_local().await?;
        ensure!(
            endpoint.typ() == EndpointType::Path,
            UiError::InvalidFSEndpoint.msg("use path style endpoint for FS setup".to_owned())
        );
        endpoints.push(endpoint);
        let endpoints = Endpoints(endpoints);
        // Check for cross device mounts if any.
        endpoints
            .check_cross_device_mounts()
            .await
            .map_err(|e| UiError::InvalidFSEndpoint.msg(e.to_string()))?;

        return Ok((endpoints, SetupType::Fs));
    }

    for &args in args_list {
        let eps = Endpoints::new(args)
            .map_err(|e| UiError::InvalidErasureEndpoints.msg(e.to_string()))?;
        eps.check_cross_device_mounts()
            .await
            .map_err(|e| UiError::InvalidErasureEndpoints.msg(e.to_string()))?;
        endpoints.extend(eps.0.into_iter());
    }

    ensure!(
        !endpoints.is_empty(),
        UiError::InvalidErasureEndpoints.msg("invalid number of endpoints".to_owned())
    );

    if endpoints[0].typ() == EndpointType::Path {
        return Ok((Endpoints(endpoints), SetupType::Erasure));
    }

    let mut endpoints = Endpoints(endpoints);
    endpoints
        .update_is_local(found_local)
        .map_err(|e| UiError::InvalidErasureEndpoints.msg(e.to_string()))?;

    let mut endpoint_path_set = StringSet::new();
    let mut local_endpoint_count = 0;
    let mut local_server_host_set = StringSet::new();
    let mut local_port_set = StringSet::new();

    for endpoint in &endpoints.0 {
        endpoint_path_set.add(endpoint.url.path().to_owned());
        if endpoint.is_local {
            local_server_host_set.add(endpoint.url.host_str().unwrap().to_owned());
            let port = endpoint
                .url
                .port_or_known_default()
                .map(|p| p.to_string())
                .unwrap_or_else(|| server_addr_port.clone());
            local_port_set.add(port);
            local_endpoint_count += 1;
        }
    }

    {
        let mut path_ip_map: HashMap<String, StringSet> = HashMap::new();
        for endpoint in &endpoints.0 {
            let host_ips = get_host_ip(endpoint.url.host_str().unwrap()).await;
            let host_ips = host_ips.unwrap_or_else(|_| StringSet::new()); // ignore error
            if let Some(ips) = path_ip_map.get_mut(endpoint.url.path()) {
                ensure!(
                    ips.intersection(&host_ips).is_empty(),
                    UiError::InvalidErasureEndpoints.msg(format!(
                        "path '{}' can not be served by different port on same address",
                        endpoint.url.path()
                    ))
                );
                *ips = ips.union(&host_ips);
            } else {
                path_ip_map.insert(endpoint.url.path().to_owned(), host_ips);
            }
        }
    }

    {
        let mut local_path_set = StringSet::new();
        for endpoint in &endpoints.0 {
            if !endpoint.is_local {
                continue;
            }
            ensure!(
                !local_path_set.contains(endpoint.url.path()),
                UiError::InvalidErasureEndpoints.msg(format!(
                    "path '{}' cannot be served by different address on same server",
                    endpoint.url.path()
                ))
            );
            local_path_set.add(endpoint.url.path().to_owned());
        }
    }

    if endpoints.0.len() == local_endpoint_count {
        if local_port_set.len() == 1 {
            ensure!(
                local_server_host_set.len() <= 1,
                UiError::InvalidErasureEndpoints
                    .msg("all local endpoints should not have different hostnames/ips".to_owned())
            );
            return Ok((endpoints, SetupType::Erasure));
        }
    }

    for endpoint in endpoints.0.iter_mut() {
        match endpoint.url.port_or_known_default() {
            None => {
                endpoint
                    .url
                    .set_port(Some(server_addr_port.parse().unwrap()))
                    .unwrap();
            }
            Some(port) => {
                if endpoint.is_local && server_addr_port != port.to_string() {
                    endpoint.is_local = false;
                }
            }
        }
    }

    let mut unique_args = StringSet::new();
    for endpoint in &endpoints.0 {
        unique_args.add(endpoint.url.host_str().unwrap().to_owned());
    }

    ensure!(unique_args.len() >= 2, UiError::InvalidErasureEndpoints.msg(format!("Unsupported number of endpoints ({:?}), minimum number of servers cannot be less than 2 in distributed setup", endpoints.0)));

    let no_public_ips = std::env::var(crate::config::ENV_PUBLIC_IPS)
        .map(|v| v.is_empty())
        .unwrap_or(true);
    if no_public_ips {
        update_domain_ips(&unique_args).await;
    }

    Ok((endpoints, SetupType::DistributedErasure))
}

pub async fn update_domain_ips(endpoints: &StringSet) {
    let mut ip_list = StringSet::new();
    for e in endpoints.iter() {
        let host_port = split_host_port(e);
        if host_port.is_err() {
            continue;
        }
        let (host, _) = host_port.unwrap();
        if host.parse::<IpAddr>().is_err() {
            if let Ok(ips) = get_host_ip(&host).await {
                ip_list = ip_list.union(&ips);
            }
        } else {
            ip_list.add(host);
        }
    }

    *GLOBALS.domain_ips.guard() =
        ip_list.match_fn(|ip| ip.parse::<IpAddr>().unwrap().is_loopback() && ip != "localhost");
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
            /* TODO
            (
                "/foo",
                Endpoint {
                    url: url::Url::from_file_path("/foo").unwrap(),
                    is_local: true,
                },
                EndpointType::Path,
                false,
            ),
            */
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

    #[test]
    fn test_endpoints_new() {
        let cases = vec![
            (&["/d1", "/d2", "/d3", "/d4"][..], false),
            (
                &[
                    "http://localhost/d1",
                    "http://localhost/d2",
                    "http://localhost/d3",
                    "http://localhost/d4",
                ][..],
                false,
            ),
            (
                &[
                    "http://example.org/d1",
                    "http://example.com/d1",
                    "http://example.net/d1",
                    "http://example.edu/d1",
                ][..],
                false,
            ),
            (
                &[
                    "http://localhost/d1",
                    "http://localhost/d2",
                    "http://example.org/d1",
                    "http://example.org/d2",
                ][..],
                false,
            ),
            (
                &[
                    "https://localhost:9000/d1",
                    "https://localhost:9001/d2",
                    "https://localhost:9002/d3",
                    "https://localhost:9003/d4",
                ][..],
                false,
            ),
            (
                &[
                    "https://127.0.0.1:9000/d1",
                    "https://127.0.0.1:9001/d1",
                    "https://127.0.0.1:9002/d1",
                    "https://127.0.0.1:9003/d1",
                ][..],
                false,
            ),
            (&["d1", "d2", "d3", "d1"][..], true),
            (&["d1", "d2", "d3", "./d1"][..], true),
            (
                &[
                    "http://localhost/d1",
                    "http://localhost/d2",
                    "http://localhost/d1",
                    "http://localhost/d4",
                ][..],
                true,
            ),
            (
                &[
                    "ftp://server/d1",
                    "http://server/d2",
                    "http://server/d3",
                    "http://server/d4",
                ][..],
                true,
            ),
            (&["d1", "http://localhost/d2", "d3", "d4"][..], true),
            (
                &[
                    "http://example.org/d1",
                    "https://example.com/d1",
                    "http://example.net/d1",
                    "https://example.edut/d1",
                ][..],
                true,
            ),
            (
                &[
                    "192.168.1.210:9000/tmp/dir0",
                    "192.168.1.210:9000/tmp/dir1",
                    "192.168.1.210:9000/tmp/dir2",
                    "192.168.110:9000/tmp/dir3",
                ][..],
                false,
            ),
        ];
        for (args, expected_err) in cases.into_iter() {
            let endpoints = Endpoints::new(args);
            match endpoints {
                Err(err) => assert!(
                    expected_err,
                    "unexpected err: {}, {:?}",
                    err.to_string(),
                    args
                ),
                Ok(_) => assert!(!expected_err, "expected err but none occurred: {:?}", args),
            }
        }
    }
}
