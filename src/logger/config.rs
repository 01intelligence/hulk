use std::collections::HashMap;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::config::{self, KV, KVS};
use crate::utils;

#[derive(Serialize, Deserialize, Default)]
pub struct Console {
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Webhook {
    pub enabled: bool,
    pub endpoint: String,
    pub auth_token: String,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub console: Console,
    pub logger: HashMap<String, Webhook>,
    pub audit: HashMap<String, Webhook>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            console: Console { enabled: true },
            logger: Default::default(),
            audit: Default::default(),
        }
    }
}

pub const ENDPOINT: &str = "endpoint";
pub const AUTH_TOKEN: &str = "auth_token";
pub const CLIENT_CERT: &str = "client_cert";
pub const CLIENT_KEY: &str = "client_key";

pub const ENV_LOGGER_WEBHOOK_ENABLE: &str = "HULK_LOGGER_WEBHOOK_ENABLE";
pub const ENV_LOGGER_WEBHOOK_ENDPOINT: &str = "HULK_LOGGER_WEBHOOK_ENDPOINT";
pub const ENV_LOGGER_WEBHOOK_AUTH_TOKEN: &str = "HULK_LOGGER_WEBHOOK_AUTH_TOKEN";

pub const ENV_AUDIT_WEBHOOK_ENABLE: &str = "HULK_AUDIT_WEBHOOK_ENABLE";
pub const ENV_AUDIT_WEBHOOK_ENDPOINT: &str = "HULK_AUDIT_WEBHOOK_ENDPOINT";
pub const ENV_AUDIT_WEBHOOK_AUTH_TOKEN: &str = "HULK_AUDIT_WEBHOOK_AUTH_TOKEN";
pub const ENV_AUDIT_WEBHOOK_CLIENT_CERT: &str = "HULK_AUDIT_WEBHOOK_CLIENT_CERT";
pub const ENV_AUDIT_WEBHOOK_CLIENT_KEY: &str = "HULK_AUDIT_WEBHOOK_CLIENT_KEY";

lazy_static! {
    pub static ref DEFAULT_LOGGER_KVS: KVS = KVS(vec![
        KV {
            key: config::ENABLE_KEY.to_owned(),
            value: config::ENABLE_OFF.to_owned(),
        },
        KV {
            key: ENDPOINT.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: AUTH_TOKEN.to_owned(),
            value: "".to_owned(),
        },
    ]);
    pub static ref DEFAULT_AUDIT_KVS: KVS = KVS(vec![
        KV {
            key: config::ENABLE_KEY.to_owned(),
            value: config::ENABLE_OFF.to_owned(),
        },
        KV {
            key: ENDPOINT.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: AUTH_TOKEN.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: CLIENT_CERT.to_owned(),
            value: "".to_owned(),
        },
        KV {
            key: CLIENT_KEY.to_owned(),
            value: "".to_owned(),
        },
    ]);
}

pub fn lookup_config(scfg: config::Config) -> anyhow::Result<Config> {
    let mut cfg = Config::new();

    let mut logger_targets = Vec::new();
    for (key, _) in std::env::vars() {
        if let Some(target) = key.strip_prefix(ENV_LOGGER_WEBHOOK_ENDPOINT) {
            let target = match target.strip_prefix(config::DEFAULT) {
                None => config::DEFAULT,
                Some(target) => target,
            };
            logger_targets.push(target.to_owned());
        }
    }

    let mut audit_targets = Vec::new();
    for (key, _) in std::env::vars() {
        if let Some(target) = key.strip_prefix(ENV_AUDIT_WEBHOOK_ENDPOINT) {
            let target = match target.strip_prefix(config::DEFAULT) {
                None => config::DEFAULT,
                Some(target) => target,
            };
            audit_targets.push(target.to_owned());
        }
    }

    let build_env_name = |env: &str, target: &str| {
        let mut env = env.to_owned();
        if target != config::DEFAULT {
            env.push_str(config::DEFAULT);
            env.push_str(target);
        }
        env
    };

    for target in logger_targets {
        let env_enable = build_env_name(ENV_LOGGER_WEBHOOK_ENABLE, &target);
        let enable = utils::parse_bool_ext(&std::env::var(env_enable).unwrap_or_default())
            .unwrap_or_default();
        if !enable {
            continue;
        }
        let env_endpoint = build_env_name(ENV_LOGGER_WEBHOOK_ENDPOINT, &target);
        let env_auth_token = build_env_name(ENV_LOGGER_WEBHOOK_AUTH_TOKEN, &target);
        cfg.logger.insert(
            target,
            Webhook {
                enabled: true,
                endpoint: std::env::var(env_endpoint).unwrap_or_default(),
                auth_token: std::env::var(env_auth_token).unwrap_or_default(),
                ..Default::default()
            },
        );
    }

    for target in audit_targets {
        let env_enable = build_env_name(ENV_AUDIT_WEBHOOK_ENABLE, &target);
        let enable = utils::parse_bool_ext(&std::env::var(env_enable).unwrap_or_default())
            .unwrap_or_default();
        if !enable {
            continue;
        }
        let env_endpoint = build_env_name(ENV_AUDIT_WEBHOOK_ENDPOINT, &target);
        let env_auth_token = build_env_name(ENV_AUDIT_WEBHOOK_AUTH_TOKEN, &target);
        let env_client_cert = build_env_name(ENV_AUDIT_WEBHOOK_CLIENT_CERT, &target);
        let env_client_key = build_env_name(ENV_AUDIT_WEBHOOK_CLIENT_KEY, &target);
        let client_cert = Some(std::env::var(env_client_cert).unwrap_or_default());
        let client_key = Some(std::env::var(env_client_key).unwrap_or_default());
        ensure_cert_key(client_cert.as_ref().unwrap(), client_key.as_ref().unwrap())?;
        cfg.audit.insert(
            target,
            Webhook {
                enabled: true,
                endpoint: std::env::var(env_endpoint).unwrap_or_default(),
                auth_token: std::env::var(env_auth_token).unwrap_or_default(),
                client_cert,
                client_key,
            },
        );
    }

    if let Some(scfg) = scfg.get(config::LOGGER_WEBHOOK_SUB_SYS) {
        for (target, kv) in scfg {
            if cfg.logger.get(target).filter(|l| l.enabled).is_some() {
                // Ignore this target since there is a target with the same name
                // loaded and enabled from the environment.
                continue;
            }
            let mut sub_sys_target = config::LOGGER_WEBHOOK_SUB_SYS.to_owned();
            if target != config::DEFAULT {
                sub_sys_target.push_str(config::SUB_SYSTEM_SEPARATOR);
                sub_sys_target.push_str(target);
            }
            config::check_valid_keys(&sub_sys_target, kv, &*DEFAULT_LOGGER_KVS)?;
            let enable = utils::parse_bool_ext(kv.get(config::ENABLE_KEY))?;
            if !enable {
                continue;
            }
            cfg.logger.insert(
                target.to_owned(),
                Webhook {
                    enabled: true,
                    endpoint: kv.get(ENDPOINT).to_owned(),
                    auth_token: kv.get(AUTH_TOKEN).to_owned(),
                    ..Default::default()
                },
            );
        }
    }

    if let Some(scfg) = scfg.get(config::AUDIT_WEBHOOK_SUB_SYS) {
        for (target, kv) in scfg {
            if cfg.logger.get(target).filter(|l| l.enabled).is_some() {
                // Ignore this target since there is a target with the same name
                // loaded and enabled from the environment.
                continue;
            }
            let mut sub_sys_target = config::AUDIT_WEBHOOK_SUB_SYS.to_owned();
            if target != config::DEFAULT {
                sub_sys_target.push_str(config::SUB_SYSTEM_SEPARATOR);
                sub_sys_target.push_str(target);
            }
            config::check_valid_keys(&sub_sys_target, kv, &*DEFAULT_AUDIT_KVS)?;
            let enable = utils::parse_bool_ext(kv.get(config::ENABLE_KEY))?;
            if !enable {
                continue;
            }
            let client_cert = Some(kv.get(CLIENT_CERT).to_owned());
            let client_key = Some(kv.get(CLIENT_KEY).to_owned());
            ensure_cert_key(client_cert.as_ref().unwrap(), client_key.as_ref().unwrap())?;
            cfg.logger.insert(
                target.to_owned(),
                Webhook {
                    enabled: true,
                    endpoint: kv.get(ENDPOINT).to_owned(),
                    auth_token: kv.get(AUTH_TOKEN).to_owned(),
                    client_cert,
                    client_key,
                },
            );
        }
    }

    Ok(cfg)
}

fn ensure_cert_key(cert: &str, key: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        (cert.is_empty() && key.is_empty()) || (!cert.is_empty() && !key.is_empty()),
        "cert and key must be specified as a pair"
    );
    Ok(())
}
