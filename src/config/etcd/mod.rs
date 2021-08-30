use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;
use crate::utils;
use crate::utils::Duration;

mod help;

pub use help::*;

// Default values used while communicating with etcd.
const DEFAULT_DIAL_TIMEOUT: Duration = utils::seconds(5);
const DEFAULT_DIAL_KEEP_ALIVE: Duration = utils::seconds(30);

pub const ENDPOINTS: &str = "endpoints";
pub const PATH_PREFIX: &str = "path_prefix";
pub const CORE_DNS_PATH: &str = "coredns_path";
pub const CLIENT_CERT: &str = "client_cert";
pub const CLIENT_CERT_KEY: &str = "client_cert_key";

pub const ENV_ETCD_ENDPOINTS: &str = "HULK_ETCD_ENDPOINTS";
pub const ENV_ETCD_PATH_PREFIX: &str = "HULK_ETCD_PATH_PREFIX";
pub const ENV_ETCD_CORE_DNSPATH: &str = "HULK_ETCD_COREDNS_PATH";
pub const ENV_ETCD_CLIENT_CERT: &str = "HULK_ETCD_CLIENT_CERT";
pub const ENV_ETCD_CLIENT_CERT_KEY: &str = "HULK_ETCD_CLIENT_CERT_KEY";

lazy_static! {
    // Default storage class config
    pub static ref DEFAULT_KVS: KVS = KVS(vec![
        KV {
            key: ENDPOINTS.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: PATH_PREFIX.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: CORE_DNS_PATH.to_owned(),
            value: "/skydns".to_owned(),
        },
        KV {
            key: CLIENT_CERT.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: CLIENT_CERT_KEY.to_owned(),
            value: "".to_owned(),
        },
    ]);
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub enabled: bool,
    pub path_prefix: String,
    pub core_dns_path: String,
}
