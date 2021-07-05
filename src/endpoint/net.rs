use std::cmp::Ordering;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use anyhow::ensure;

use crate::errors;
use crate::globals::*;
use crate::strset::StringSet;

pub fn split_host_port(host_port: &str) -> anyhow::Result<(String, String)> {
    let url = url::Url::parse(host_port)?;
    ensure!(
        url.scheme().is_empty() || url.scheme() == "http" || url.scheme() == "https",
        errors::UiErrorInvalidAddressFlag.clone()
    );
    ensure!(
        !url.has_authority(),
        errors::UiErrorInvalidAddressFlag.clone()
    ); // no username/password
    ensure!(
        (&url[url::Position::BeforePath..]).is_empty(),
        errors::UiErrorInvalidAddressFlag.clone()
    ); // no path/query/fragment
    let host = url.host_str().unwrap_or("").to_owned();
    let port = url
        .port_or_known_default()
        .map_or("".to_owned(), |p| p.to_string()); // allow empty port
    Ok((host, port))
}

pub fn get_local_ip4() -> StringSet {
    let interfaces = pnet::datalink::interfaces();
    let mut ip_list = StringSet::new();
    for inf in interfaces {
        for ip in inf.ips {
            if let IpAddr::V4(ip) = ip.ip() {
                ip_list.add(ip.to_string());
            }
        }
    }
    ip_list
}

pub fn get_local_ip6() -> StringSet {
    let interfaces = pnet::datalink::interfaces();
    let mut ip_list = StringSet::new();
    for inf in interfaces {
        for ip in inf.ips {
            if let IpAddr::V6(ip) = ip.ip() {
                ip_list.add(ip.to_string());
            }
        }
    }
    ip_list
}

pub async fn get_host_ip(host: &str) -> anyhow::Result<StringSet> {
    let mut ip_list = StringSet::new();
    for addr in tokio::net::lookup_host(host).await? {
        ip_list.add(addr.ip().to_string())
    }
    Ok(ip_list)
}

pub fn sort_ips(ip_list: &[&str]) -> Vec<String> {
    let mut v4_ips = Vec::new();
    let mut non_ips = Vec::new();
    for &ip in ip_list {
        if let Ok(IpAddr::V4(ip)) = ip.parse::<IpAddr>() {
            v4_ips.push(ip);
        } else {
            non_ips.push(ip.to_owned());
        }
    }
    v4_ips.sort_unstable_by(|a, b| {
        if a.is_loopback() {
            return Ordering::Less;
        }
        if b.is_loopback() {
            return Ordering::Greater;
        }
        return b.octets()[3].cmp(&a.octets()[3]);
    });
    let mut ip_list = non_ips;
    for ip in v4_ips {
        ip_list.push(ip.to_string());
    }
    ip_list
}

pub fn get_api_endpoints() -> Vec<String> {
    let mut ip_list;
    let global_host = GLOBAL_HOST.lock().unwrap();
    if global_host.is_empty() {
        ip_list = sort_ips(&get_local_ip4().as_slice());
        ip_list.extend(get_local_ip6().into_iter());
    } else {
        ip_list = vec![global_host.clone()];
    }
    let global_port = GLOBAL_PORT.lock().unwrap();
    ip_list
        .iter()
        .map(|ip| {
            let socket = (ip as &str, global_port.parse::<u16>().unwrap()).to_socket_addrs();
            format!("{}://{}", get_url_scheme(), socket.unwrap().next().unwrap(),)
        })
        .collect()
}

pub fn is_host_ip(ip_addr: &str) -> bool {
    let host = split_host_port(ip_addr)
        .map(|(host, _)| host)
        .unwrap_or_else(|_| ip_addr.to_owned());
    host.parse::<IpAddr>().is_ok()
}

pub async fn check_port_availability(host: &str, port: &str) -> anyhow::Result<()> {
    let _ = tokio::net::TcpListener::bind((host, port.parse::<u16>()?)).await?;
    Ok(())
}

pub async fn is_local_host(host: &str, port: &str, local_port: &str) -> anyhow::Result<bool> {
    let mut host_ips = get_host_ip(host).await?;
    let mut local_v4_ips = get_local_ip4().intersection(&host_ips);
    if local_v4_ips.is_empty() {
        host_ips = host_ips.apply_fn(|ip| {
            let ip: IpAddr = ip.parse().unwrap();
            if ip.is_loopback() {
                // For any loopback IP which is not 127.0.0.1,
                // convert it to check for intersections.
                return "127.0.0.1".to_owned();
            }
            return ip.to_string();
        });
        local_v4_ips = get_local_ip4().intersection(&host_ips);
    }
    let local_v6_ips = get_local_ip6().intersection(&host_ips);

    Ok((!local_v4_ips.is_empty() || !local_v6_ips.is_empty())
        && (port.is_empty() || port == local_port))
}

pub async fn check_local_server_addr(server_addr: &str) -> anyhow::Result<()> {
    let (host, _) = split_host_port(server_addr)?;
    if !host.is_empty()
        && host != Ipv4Addr::UNSPECIFIED.to_string()
        && host != Ipv6Addr::UNSPECIFIED.to_string()
    {
        let local = is_local_host(&host, "", "").await?;
        ensure!(
            local,
            errors::UiErrorInvalidAddressFlag
                .msg("host in server address should be this server".to_owned())
        );
    }
    Ok(())
}
