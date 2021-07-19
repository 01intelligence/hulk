use std::time::Duration;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::*;

pub const DELAY: &str = "delay";
pub const MAX_WAIT: &str = "max_wait";
pub const CYCLE: &str = "cycle";

pub const ENV_DELAY: &str = "HULK_SCANNER_DELAY";
pub const ENV_CYCLE: &str = "HULK_SCANNER_CYCLE";
pub const ENV_MAX_WAIT: &str = "HULK_SCANNER_MAX_WAIT";

lazy_static! {
    pub static ref DEFAULT_KVS: KVS = KVS(vec![
        KV {
            key: DELAY.to_owned(),
            value: "10".to_owned(),
        },
        KV {
            key: MAX_WAIT.to_owned(),
            value: "15s".to_owned(),
        },
        KV {
            key: CYCLE.to_owned(),
            value: "1m".to_owned(),
        },
    ]);
    pub static ref HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: DELAY.to_owned(),
            description: "scanner delay multiplier, defaults to '10.0'".to_owned(),
            optional: true,
            typ: "float".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: MAX_WAIT.to_owned(),
            description: "maximum wait time between operations, defaults to '15s'".to_owned(),
            optional: true,
            typ: "duration".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: CYCLE.to_owned(),
            description: "time duration between scanner cycles, defaults to '1m'".to_owned(),
            optional: true,
            typ: "duration".to_owned(),
            ..Default::default()
        },
    ]);
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    // The sleep multiplier.
    pub delay: f64,
    // The maximum wait time between operations.
    pub max_wait: Duration,
    // The duration between each scanner cycles.
    pub cycle: Duration,
}

pub fn lookup_config(kvs: &KVS) -> anyhow::Result<Config> {
    let _ = check_valid_keys(API_SUB_SYS, kvs, &DEFAULT_KVS)?;

    let delay = std::env::var(ENV_DELAY).unwrap_or_else(|_| kvs.get(DELAY).to_owned());
    let delay = delay.parse::<f64>()?;

    let max_wait = std::env::var(ENV_MAX_WAIT).unwrap_or_else(|_| kvs.get(MAX_WAIT).to_owned());
    let max_wait = humantime::parse_duration(&max_wait)?;

    let cycle = std::env::var(ENV_CYCLE).unwrap_or_else(|_| kvs.get(CYCLE).to_owned());
    let cycle = humantime::parse_duration(&cycle)?;

    Ok(Config {
        delay,
        max_wait,
        cycle,
    })
}
