use std::time::Duration;

use anyhow::{bail, ensure};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::config::{KV, KVS};

mod help;

pub use help::*;

use crate::config::{check_valid_keys, API_SUB_SYS};

pub(self) const API_REQUESTS_MAX: &str = "requests_max";
pub(self) const API_REQUESTS_DEADLINE: &str = "requests_deadline";
pub(self) const API_CLUSTER_DEADLINE: &str = "cluster_deadline";
pub(self) const API_CORS_ALLOW_ORIGIN: &str = "cors_allow_origin";
pub(self) const API_REMOTE_TRANSPORT_DEADLINE: &str = "remote_transport_deadline";
pub(self) const API_LIST_QUORUM: &str = "list_quorum";
pub(self) const API_EXTEND_LIST_CACHE_LIFE: &str = "extend_list_cache_life";
pub(self) const API_REPLICATION_WORKERS: &str = "replication_workers";
pub(self) const API_REPLICATION_FAILED_WORKERS: &str = "replication_failed_workers";

pub const ENV_API_REQUESTS_MAX: &str = "HULK_API_REQUESTS_MAX";
pub const ENV_API_REQUESTS_DEADLINE: &str = "HULK_API_REQUESTS_DEADLINE";
pub const ENV_API_CLUSTER_DEADLINE: &str = "HULK_API_CLUSTER_DEADLINE";
pub const ENV_API_CORS_ALLOW_ORIGIN: &str = "HULK_API_CORS_ALLOW_ORIGIN";
pub const ENV_API_REMOTE_TRANSPORT_DEADLINE: &str = "HULK_API_REMOTE_TRANSPORT_DEADLINE";
pub const ENV_API_LIST_QUORUM: &str = "HULK_API_LIST_QUORUM";
pub const ENV_API_EXTEND_LIST_CACHE_LIFE: &str = "HULK_API_EXTEND_LIST_CACHE_LIFE";
pub const ENV_API_SECURE_CIPHERS: &str = "HULK_API_SECURE_CIPHERS";
pub const ENV_API_REPLICATION_WORKERS: &str = "HULK_API_REPLICATION_WORKERS";
pub const ENV_API_REPLICATION_FAILED_WORKERS: &str = "HULK_API_REPLICATION_FAILED_WORKERS";

lazy_static! {
    pub static ref DEFAULT_KVS: KVS = KVS(vec![
        KV {
            key: API_REQUESTS_MAX.to_owned(),
            value: "0".to_owned(),
        },
        KV {
            key: API_REQUESTS_DEADLINE.to_owned(),
            value: "10s".to_owned(),
        },
        KV {
            key: API_CLUSTER_DEADLINE.to_owned(),
            value: "10s".to_owned(),
        },
        KV {
            key: API_CORS_ALLOW_ORIGIN.to_owned(),
            value: "*".to_owned(),
        },
        KV {
            key: API_REMOTE_TRANSPORT_DEADLINE.to_owned(),
            value: "2h".to_owned(),
        },
        KV {
            key: API_LIST_QUORUM.to_owned(),
            value: "optimal".to_owned(),
        },
        KV {
            key: API_EXTEND_LIST_CACHE_LIFE.to_owned(),
            value: "0s".to_owned(),
        },
        KV {
            key: API_REPLICATION_WORKERS.to_owned(),
            value: "250".to_owned(),
        },
        KV {
            key: API_REPLICATION_FAILED_WORKERS.to_owned(),
            value: "8".to_owned(),
        },
    ]);
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub requests_max: usize,
    pub requests_deadline: Duration,
    pub cluster_deadline: Duration,
    pub cors_allow_origin: Vec<String>,
    pub remote_transport_deadline: Duration,
    pub list_quorum: String,
    pub extend_list_cache_life: Duration,
    pub replication_workers: usize,
    pub replication_failed_workers: usize,
}

impl Config {
    // Interprets list quorum values and returns appropriate
    // acceptable quorum expected for list operations
    pub fn get_list_quorum(&self) -> isize {
        match &self.list_quorum as &str {
            "reduced" => 2,
            "disk" => 1, // smallest possible value, generally meant for testing
            "strict" => -1,
            _ => 3, // defaults to 3 drives per set, defaults to "optimal" value
        }
    }
}

pub fn lookup_config(kvs: &KVS) -> anyhow::Result<Config> {
    let _ = check_valid_keys(API_SUB_SYS, kvs, &DEFAULT_KVS)?;

    let requests_max = std::env::var(ENV_API_REQUESTS_MAX)
        .unwrap_or_else(|_| kvs.get(API_REQUESTS_MAX).to_owned());
    let requests_max = requests_max.parse::<usize>()?;

    let requests_deadline = std::env::var(ENV_API_REQUESTS_DEADLINE)
        .unwrap_or_else(|_| kvs.get(API_REQUESTS_DEADLINE).to_owned());
    let requests_deadline = humantime::parse_duration(&requests_deadline)?;

    let cluster_deadline = std::env::var(ENV_API_CLUSTER_DEADLINE)
        .unwrap_or_else(|_| kvs.get(API_CLUSTER_DEADLINE).to_owned());
    let cluster_deadline = humantime::parse_duration(&cluster_deadline)?;

    let cors_allow_origin = std::env::var(ENV_API_CORS_ALLOW_ORIGIN)
        .unwrap_or_else(|_| kvs.get(API_CORS_ALLOW_ORIGIN).to_owned());
    let cors_allow_origin: Vec<_> = cors_allow_origin.split(',').map(|s| s.to_owned()).collect();

    let remote_transport_deadline = std::env::var(ENV_API_REMOTE_TRANSPORT_DEADLINE)
        .unwrap_or_else(|_| kvs.get(API_REMOTE_TRANSPORT_DEADLINE).to_owned());
    let remote_transport_deadline = humantime::parse_duration(&remote_transport_deadline)?;

    let list_quorum =
        std::env::var(ENV_API_LIST_QUORUM).unwrap_or_else(|_| kvs.get(API_LIST_QUORUM).to_owned());
    match &list_quorum as &str {
        "strict" | "optimal" | "reduced" | "disk" => {}
        _ => {
            bail!("invalid value for list quorum");
        }
    }

    let extend_list_cache_life = std::env::var(ENV_API_EXTEND_LIST_CACHE_LIFE)
        .unwrap_or_else(|_| kvs.get(API_EXTEND_LIST_CACHE_LIFE).to_owned());
    let extend_list_cache_life = humantime::parse_duration(&extend_list_cache_life)?;

    let replication_workers = std::env::var(ENV_API_REPLICATION_WORKERS)
        .unwrap_or_else(|_| kvs.get(API_REPLICATION_WORKERS).to_owned());
    let replication_workers = replication_workers.parse::<usize>()?;
    ensure!(
        replication_workers > 0,
        crate::errors::UiError::InvalidReplicationWorkersValue
            .msg("Minimum number of replication workers should be 1".to_owned())
    );

    let replication_failed_workers = std::env::var(ENV_API_REPLICATION_FAILED_WORKERS)
        .unwrap_or_else(|_| kvs.get(API_REPLICATION_FAILED_WORKERS).to_owned());
    let replication_failed_workers = replication_failed_workers.parse::<usize>()?;
    ensure!(
        replication_failed_workers > 0,
        crate::errors::UiError::InvalidReplicationWorkersValue
            .msg("Minimum number of replication failed workers should be 1".to_owned())
    );

    Ok(Config {
        requests_max,
        requests_deadline,
        cluster_deadline,
        cors_allow_origin,
        remote_transport_deadline,
        list_quorum,
        extend_list_cache_life,
        replication_workers,
        replication_failed_workers,
    })
}
