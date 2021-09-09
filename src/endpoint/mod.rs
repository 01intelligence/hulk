use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;

use anyhow::{anyhow, ensure};

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

use crate::prelude::*;

#[derive(Debug, Eq, PartialEq)]
pub enum EndpointType {
    Path,
    Url,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Endpoint {
    Path(PathBuf),
    Url(url::Url, bool),
}

#[derive(Debug)]
pub struct Endpoints(Vec<Endpoint>);

pub struct PoolEndpoints {
    pub set_count: usize,
    pub drives_per_set: usize,
    pub endpoints: Endpoints,
}

#[derive(Default)]
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
            // URL style of endpoint.
            let mut url = url.unwrap();
            // Valid URL style endpoint is
            // - Scheme field must contain "http" or "https"
            // - All fields should be empty except `host` and `path`.
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

            Ok(Endpoint::Url(url, false))
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
            let path = Path::new(arg).absolutize()?.clean();
            Ok(Endpoint::Path(path))
        }
    }

    pub fn typ(&self) -> EndpointType {
        match self {
            Endpoint::Path(_) => EndpointType::Path,
            Endpoint::Url(_, _) => EndpointType::Url,
        }
    }

    pub fn scheme(&self) -> &str {
        if let Endpoint::Url(url, _) = self {
            url.scheme()
        } else {
            ""
        }
    }

    pub fn is_https(&self) -> bool {
        if let Endpoint::Url(url, _) = self {
            url.scheme() == "https"
        } else {
            false
        }
    }

    pub fn host(&self) -> &str {
        if let Endpoint::Url(url, _) = self {
            url.host_str().unwrap_or_default()
        } else {
            ""
        }
    }

    pub fn port(&self) -> Option<u16> {
        if let Endpoint::Url(url, _) = self {
            url.port_or_known_default()
        } else {
            None
        }
    }

    pub fn set_port(&mut self, port: u16) {
        if let Endpoint::Url(url, _) = self {
            let _ = url.set_port(Some(port));
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Endpoint::Path(path) => path.as_str(),
            Endpoint::Url(url, _) => url.path(),
        }
    }

    pub fn is_local(&self) -> bool {
        match self {
            Endpoint::Path(_) => true,
            Endpoint::Url(_, is_local) => *is_local,
        }
    }

    pub async fn update_is_local(&mut self) -> anyhow::Result<()> {
        if !self.is_local() {
            if let Endpoint::Url(url, is_local) = self {
                *is_local = is_local_host(
                    url.host_str().unwrap(),
                    &url.port().map(|p| p.to_string()).unwrap_or_default(),
                    &*GLOBALS.port.guard(),
                )
                .await?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Endpoint::Path(path) => path.fmt(f),
            Endpoint::Url(url, _) => {
                if !url.has_host() {
                    url.path().fmt(f)
                } else {
                    url.to_string().fmt(f)
                }
            }
        }
    }
}

impl Endpoints {
    pub fn iter(&self) -> std::slice::Iter<'_, Endpoint> {
        self.0.iter()
    }

    pub fn new(args: &Vec<String>) -> anyhow::Result<Endpoints> {
        let mut endpoint_type = None;
        let mut scheme = None;
        let mut unique_args = StringSet::new();
        let mut endpoints = Vec::new();
        for (i, arg) in args.iter().enumerate() {
            let endpoint = Endpoint::new(arg)?;
            if i == 0 {
                endpoint_type = Some(endpoint.typ());
                scheme = Some(endpoint.scheme().to_owned());
            } else {
                ensure!(
                    Some(endpoint.typ()) == endpoint_type,
                    "mixed style endpoints are not supported"
                );
                ensure!(
                    endpoint.scheme() == scheme.as_ref().unwrap(),
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

    pub fn at_least_one_endpoiont_local(&self) -> bool {
        for endpoint in self.iter() {
            if endpoint.is_local() {
                return true;
            }
        }
        return false;
    }

    async fn check_cross_device_mounts(&self) -> anyhow::Result<()> {
        let mut abs_paths = Vec::new();
        for e in &self.0 {
            if e.is_local() {
                abs_paths.push(Path::new(e.path()).absolutize()?)
            }
        }
        crate::mount::check_cross_device(&abs_paths)
    }

    pub fn update_is_local(&mut self, found_prev_local: bool) -> anyhow::Result<()> {
        todo!()
    }
}

impl EndpointServerPools {
    pub fn iter(&self) -> std::slice::Iter<'_, PoolEndpoints> {
        self.0.iter()
    }

    pub fn get_local_pool_idx(&self, endpoint: &Endpoint) -> isize {
        for (i, p) in self.0.iter().enumerate() {
            for e in &p.endpoints.0 {
                if e.is_local() && endpoint.is_local() && e == endpoint {
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
                if e.is_local() {
                    return format!("{}://{}", e.scheme(), e.host());
                }
            }
        }
        "".to_owned()
    }

    pub fn local_disk_paths(&self) -> Vec<String> {
        let mut disks = Vec::new();
        for p in &self.0 {
            for e in &p.endpoints.0 {
                if e.is_local() {
                    disks.push(e.path().to_owned());
                }
            }
        }
        disks
    }

    pub fn first_local(&self) -> bool {
        self.0[0].endpoints.0[0].is_local()
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
                let host = e.host();
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
                let host = e.host().to_owned();
                let port = e.port().unwrap();
                let peer = crate::net::Host::new(host, Some(port)).to_string();
                all.add(peer.clone());
                if e.is_local() {
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
                if endpoint.is_local() && !endpoint.host().is_empty() {
                    peers.add(endpoint.host().to_owned());
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
    args_list: &Vec<Vec<String>>,
) -> anyhow::Result<(Endpoints, SetupType)> {
    check_local_server_addr(server_addr).await?;

    let (_, server_addr_port) = split_host_port(server_addr).unwrap();

    let mut endpoints = Vec::new();

    // For single arg, return FS setup.
    if args_list.len() == 1 && args_list[0].len() == 1 {
        let mut endpoint = Endpoint::new(&args_list[0][0])?;
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

    for args in args_list {
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
        endpoint_path_set.add(endpoint.path().to_owned());
        if endpoint.is_local() {
            local_server_host_set.add(endpoint.host().to_owned());
            let port = endpoint
                .port()
                .map(|p| p.to_string())
                .unwrap_or_else(|| server_addr_port.clone());
            local_port_set.add(port);
            local_endpoint_count += 1;
        }
    }

    {
        let mut path_ip_map: HashMap<String, StringSet> = HashMap::new();
        for endpoint in &endpoints.0 {
            let host_ips = get_host_ip(endpoint.host()).await;
            let host_ips = host_ips.unwrap_or_else(|_| StringSet::new()); // ignore error
            if let Some(ips) = path_ip_map.get_mut(endpoint.path()) {
                ensure!(
                    ips.intersection(&host_ips).is_empty(),
                    UiError::InvalidErasureEndpoints.msg(format!(
                        "path '{}' can not be served by different port on same address",
                        endpoint.path()
                    ))
                );
                *ips = ips.union(&host_ips);
            } else {
                path_ip_map.insert(endpoint.path().to_owned(), host_ips);
            }
        }
    }

    {
        let mut local_path_set = StringSet::new();
        for endpoint in &endpoints.0 {
            if !endpoint.is_local() {
                continue;
            }
            ensure!(
                !local_path_set.contains(endpoint.path()),
                UiError::InvalidErasureEndpoints.msg(format!(
                    "path '{}' cannot be served by different address on same server",
                    endpoint.path()
                ))
            );
            local_path_set.add(endpoint.path().to_owned());
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
        match endpoint.port() {
            None => endpoint.set_port(server_addr_port.parse().unwrap()),
            Some(port) => {
                if endpoint.is_local() && server_addr_port != port.to_string() {
                    if let Endpoint::Url(_, is_local) = endpoint {
                        *is_local = false;
                    }
                }
            }
        }
    }

    let mut unique_args = StringSet::new();
    for endpoint in &endpoints.0 {
        unique_args.add(endpoint.host().to_owned());
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
        if let Ok((host, mut port)) = split_host_port(e) {
            if port.is_empty() {
                port = GLOBALS.port.guard().clone();
            };
            match host.parse::<IpAddr>() {
                Ok(_) => ip_list.add(join_host_port(&host, &port)),
                Err(_) => {
                    if let Ok(ips) = get_host_ip(&host).await {
                        let ips_with_port = ips.apply_fn(|ip| join_host_port(ip, &port));
                        ip_list = ip_list.union(&ips_with_port);
                    }
                }
            }
        }
    }

    *GLOBALS.domain_ips.guard() = ip_list.match_fn(|ip| {
        let host_port = split_host_port(ip);
        let ip = if let Ok((ref host, _)) = host_port {
            host
        } else {
            ip
        };
        let ip_res = ip.parse::<IpAddr>();
        ip_res.is_ok() && !ip_res.unwrap().is_loopback() && ip != "localhost"
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_new() {
        let cases = vec![
            // Test 1
            (
                "/foo",
                Ok(Endpoint::Path(
                    Path::new("/foo").absolutize().unwrap().clean(),
                )),
            ),
            // Test 2
            (
                "https://example.org/path",
                Ok(Endpoint::Url(
                    url::Url::parse("https://example.org/path").unwrap(),
                    false,
                )),
            ),
            // Test 3
            (
                "http://192.168.253.200/path",
                Ok(Endpoint::Url(
                    url::Url::parse("http://192.168.253.200/path").unwrap(),
                    false,
                )),
            ),
            // Test 4
            ("", Err(anyhow!("empty or root endpoint is not supported"))),
            // Test 5
            (
                SLASH_SEPARATOR,
                Err(anyhow!("empty or root endpoint is not supported")),
            ),
            // Test 6
            (
                "\\",
                Err(anyhow!("empty or root endpoint is not supported")),
            ),
            // Test 7
            ("c://foo", Err(anyhow!("invalid URL endpoint format"))),
            // Test 8
            ("ftp://foo", Err(anyhow!("invalid URL endpoint format"))),
            // Test 9
            (
                "http://server/path?location",
                Err(anyhow!("invalid URL endpoint format")),
            ),
            // Test 10
            ("http://:/path", Err(anyhow!(url::ParseError::EmptyHost))),
            // Test 11
            (
                "http://:8080/path",
                Err(anyhow!(url::ParseError::EmptyHost)),
            ),
            // Test 12
            (
                "http://server:/path",
                Ok(Endpoint::Url(
                    url::Url::parse("http://server:80/path").unwrap(),
                    false,
                )),
            ),
            // Test 13
            (
                "https://93.184.216.34:808080/path",
                Err(anyhow!(url::ParseError::InvalidPort)),
            ),
            // Test 14
            (
                "http://server:8080//",
                Err(anyhow!(
                    "empty or root path is not supported in URL endpoint"
                )),
            ),
            // Test 15
            (
                "http://server:8080/",
                Err(anyhow!(
                    "empty or root path is not supported in URL endpoint"
                )),
            ),
            // Test 16
            (
                "192.168.1.210:9000",
                Err(anyhow!(
                    "invalid URL endpoint format: missing scheme http or https"
                )),
            ),
        ];

        for (i, (arg, expected_res)) in cases.into_iter().enumerate() {
            let res = Endpoint::new(arg).map(|mut endpoint| {
                endpoint.update_is_local();
                endpoint
            });

            match res {
                Err(err) => assert_eq!(
                    err.to_string(),
                    expected_res.unwrap_err().to_string(),
                    "test {}",
                    i + 1
                ),
                Ok(endpoint) => assert_eq!(endpoint, expected_res.unwrap(), "test {}", i + 1),
            }
        }
    }

    #[test]
    fn test_endpoints_new() {
        let cases = vec![
            // Test 1
            (
                vec![
                    "/d1".to_string(),
                    "/d2".to_string(),
                    "/d3".to_string(),
                    "/d4".to_string(),
                ],
                None,
            ),
            // Test 2
            (
                vec![
                    "http://localhost/d1".to_string(),
                    "http://localhost/d2".to_string(),
                    "http://localhost/d3".to_string(),
                    "http://localhost/d4".to_string(),
                ],
                None,
            ),
            // Test 3
            (
                vec![
                    "http://example.org/d1".to_string(),
                    "http://example.com/d1".to_string(),
                    "http://example.net/d1".to_string(),
                    "http://example.edu/d1".to_string(),
                ],
                None,
            ),
            // Test 4
            (
                vec![
                    "http://localhost/d1".to_string(),
                    "http://localhost/d2".to_string(),
                    "http://example.org/d1".to_string(),
                    "http://example.org/d2".to_string(),
                ],
                None,
            ),
            // Test 5
            (
                vec![
                    "https://localhost:9000/d1".to_string(),
                    "https://localhost:9001/d2".to_string(),
                    "https://localhost:9002/d3".to_string(),
                    "https://localhost:9003/d4".to_string(),
                ],
                None,
            ),
            // Test 6
            (
                vec![
                    "https://127.0.0.1:9000/d1".to_string(),
                    "https://127.0.0.1:9001/d1".to_string(),
                    "https://127.0.0.1:9002/d1".to_string(),
                    "https://127.0.0.1:9003/d1".to_string(),
                ],
                None,
            ),
            // Test 7
            (
                vec![
                    "d1".to_string(),
                    "d2".to_string(),
                    "d3".to_string(),
                    "d1".to_string(),
                ],
                Some(anyhow!("duplicate endpoints found")),
            ),
            // Test 8
            (
                vec![
                    "d1".to_string(),
                    "d2".to_string(),
                    "d3".to_string(),
                    "./d1".to_string(),
                ],
                Some(anyhow!("duplicate endpoints found")),
            ),
            // Test 9
            (
                vec![
                    "http://localhost/d1".to_string(),
                    "http://localhost/d2".to_string(),
                    "http://localhost/d1".to_string(),
                    "http://localhost/d4".to_string(),
                ],
                Some(anyhow!("duplicate endpoints found")),
            ),
            // Test 10
            (
                vec![
                    "ftp://server/d1".to_string(),
                    "http://server/d2".to_string(),
                    "http://server/d3".to_string(),
                    "http://server/d4".to_string(),
                ],
                Some(anyhow!("invalid URL endpoint format")),
            ),
            // Test 11
            (
                vec![
                    "d1".to_string(),
                    "http://localhost/d2".to_string(),
                    "d3".to_string(),
                    "d4".to_string(),
                ],
                Some(anyhow!("mixed style endpoints are not supported")),
            ),
            // Test 12
            (
                vec![
                    "http://example.org/d1".to_string(),
                    "https://example.com/d1".to_string(),
                    "http://example.net/d1".to_string(),
                    "https://example.edut/d1".to_string(),
                ],
                Some(anyhow!("mixed scheme is not supported")),
            ),
            // Test 13
            (
                vec![
                    "192.168.1.210:9000/tmp/dir0".to_string(),
                    "192.168.1.210:9000/tmp/dir1".to_string(),
                    "192.168.1.210:9000/tmp/dir2".to_string(),
                    "192.168.110:9000/tmp/dir3".to_string(),
                ],
                Some(anyhow!(
                    "invalid URL endpoint format: missing scheme http or https"
                )),
            ),
        ];

        for (i, (args, expected_err)) in cases.into_iter().enumerate() {
            match Endpoints::new(&args) {
                Err(err) => assert_eq!(
                    err.to_string(),
                    expected_err.unwrap().to_string(),
                    "test {}",
                    i + 1
                ),
                Ok(endpoints) => assert!(
                    expected_err.is_none(),
                    "test {} expected: {}, got: {:?}",
                    i + 1,
                    expected_err.unwrap(),
                    endpoints,
                ),
            }
        }
    }

    #[tokio::test]
    async fn test_update_domain_ips() {
        let temp_global_port = GLOBALS.port.guard().clone();
        *GLOBALS.port.guard() = "9000".to_string();
        let temp_global_domain_ips = GLOBALS.domain_ips.guard().clone();

        let cases = vec![
            // Test 1
            (StringSet::new(), StringSet::new()),
            // Test 2
            (StringSet::from_slice(&["localhost"]), StringSet::new()),
            // Test 3
            (
                StringSet::from_slice(&["localhost", "10.0.0.1"]),
                StringSet::from_slice(&["10.0.0.1:9000"]),
            ),
            // Test 4
            (
                StringSet::from_slice(&["localhost:9001", "10.0.0.1"]),
                StringSet::from_slice(&["10.0.0.1:9000"]),
            ),
            // Test 5
            (
                StringSet::from_slice(&["localhost", "10.0.0.1:9001"]),
                StringSet::from_slice(&["10.0.0.1:9001"]),
            ),
            // Test 6
            (
                StringSet::from_slice(&["localhost:9000", "10.0.0.1:9001"]),
                StringSet::from_slice(&["10.0.0.1:9001"]),
            ),
            // Test 7
            (
                StringSet::from_slice(&["10.0.0.1", "10.0.0.2"]),
                StringSet::from_slice(&["10.0.0.1:9000", "10.0.0.2:9000"]),
            ),
            // Test 8
            (
                StringSet::from_slice(&["10.0.0.1:9001", "10.0.0.2"]),
                StringSet::from_slice(&["10.0.0.1:9001", "10.0.0.2:9000"]),
            ),
            // Test 9
            (
                StringSet::from_slice(&["10.0.0.1", "10.0.0.2:9002"]),
                StringSet::from_slice(&["10.0.0.1:9000", "10.0.0.2:9002"]),
            ),
            // Test 10
            (
                StringSet::from_slice(&["10.0.0.1:9001", "10.0.0.2:9002"]),
                StringSet::from_slice(&["10.0.0.1:9001", "10.0.0.2:9002"]),
            ),
        ];

        for (i, (endpoints, expected_results)) in cases.into_iter().enumerate() {
            *GLOBALS.domain_ips.guard() = StringSet::new();
            update_domain_ips(&endpoints).await;
            assert_eq!(
                *GLOBALS.domain_ips.guard(),
                expected_results,
                "test {}",
                i + 1
            )
        }

        *GLOBALS.port.guard() = temp_global_port;
        *GLOBALS.domain_ips.guard() = temp_global_domain_ips;
    }
}
