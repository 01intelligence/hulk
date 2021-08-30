use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;
use crate::utils::Duration;

pub const BITROT: &str = "bitrotscan";
pub const SLEEP: &str = "max_sleep";
pub const IO_COUNT: &str = "max_io";

pub const ENV_BITROT: &str = "HULK_HEAL_BITROTSCAN";
pub const ENV_SLEEP: &str = "HULK_HEAL_MAX_SLEEP";
pub const ENV_IO_COUNT: &str = "HULK_HEAL_MAX_IO";

lazy_static! {
    pub static ref DEFAULT_KVS: KVS = KVS(vec![
        KV {
            key: BITROT.to_owned(),
            value: ENABLE_OFF.to_owned(),
        },
        KV {
            key: SLEEP.to_owned(),
            value: "1s".to_owned(),
        },
        KV {
            key: IO_COUNT.to_owned(),
            value: "10".to_owned(),
        },
    ]);
    pub static ref HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: BITROT.to_owned(),
            description: "perform bitrot scan on disks when checking objects during scanner"
                .to_owned(),
            optional: true,
            typ: "on|off".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: SLEEP.to_owned(),
            description:
                "maximum sleep duration between objects to slow down heal operation. eg. 2s"
                    .to_owned(),
            optional: true,
            typ: "duration".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: IO_COUNT.to_owned(),
            description:
                "maximum IO requests allowed between objects to slow down heal operation. eg. 3"
                    .to_owned(),
            optional: true,
            typ: "int".to_owned(),
            ..Default::default()
        },
    ]);
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub bitrot_scan: bool,
    pub sleep: Duration,
    pub io_count: usize,
}

pub fn lookup_config(kvs: &KVS) -> anyhow::Result<Config> {
    let _ = check_valid_keys(HEAL_SUB_SYS, kvs, &DEFAULT_KVS)?;

    let bitrot_scan = std::env::var(ENV_BITROT).unwrap_or_else(|_| kvs.get(BITROT).to_owned());
    let bitrot_scan = crate::utils::parse_bool_ext(&bitrot_scan)
        .map_err(|e| anyhow::anyhow!("heal 'bitrot_scan' value invalid: {}", e))?;

    let sleep = std::env::var(ENV_SLEEP).unwrap_or_else(|_| kvs.get(SLEEP).to_owned());
    let sleep = humantime::parse_duration(&sleep)
        .map_err(|e| anyhow::anyhow!("heal 'sleep' value invalid: {}", e))?;

    let io_count = std::env::var(ENV_IO_COUNT).unwrap_or_else(|_| kvs.get(IO_COUNT).to_owned());
    let io_count = io_count
        .parse::<usize>()
        .map_err(|e| anyhow::anyhow!("heal 'io_count' value invalid: {}", e))?;

    Ok(Config {
        bitrot_scan,
        sleep,
        io_count,
    })
}
