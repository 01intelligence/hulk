use std::collections::HashMap;
use std::ops::Index;
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, bail};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use super::help::HelpKVS;
use crate::strset::StringSet;

pub const ENABLE_KEY: &str = "enable";
pub const COMMON_KEY: &str = "comment";
pub const ENABLE_ON: &str = "on";
pub const ENABLE_OFF: &str = "off";
pub const REGION_NAME: &str = "name";
pub const ACCESS_KEY: &str = "access_key";
pub const SECRET_KEY: &str = "secret_key";

// Top level config constants.
pub const CREDENTIALS_SUB_SYS: &str = "credentials";
pub const POLICY_OPA_SUB_SYS: &str = "policy_opa";
pub const IDENTITY_OPEN_ID_SUB_SYS: &str = "identity_openid";
pub const IDENTITY_LDAP_SUB_SYS: &str = "identity_ldap";
pub const CACHE_SUB_SYS: &str = "cache";
pub const REGION_SUB_SYS: &str = "region";
pub const ETCD_SUB_SYS: &str = "etcd";
pub const STORAGE_CLASS_SUB_SYS: &str = "storage_class";
pub const API_SUB_SYS: &str = "api";
pub const COMPRESSION_SUB_SYS: &str = "compression";
pub const KMS_VAULT_SUB_SYS: &str = "kms_vault";
pub const KMS_KES_SUB_SYS: &str = "kms_kes";
pub const LOGGER_WEBHOOK_SUB_SYS: &str = "logger_webhook";
pub const AUDIT_WEBHOOK_SUB_SYS: &str = "audit_webhook";
pub const HEAL_SUB_SYS: &str = "heal";
pub const SCANNER_SUB_SYS: &str = "scanner";
// Add new constants here if you add new fields to config.

// Notification config constants.
pub const NOTIFY_KAFKA_SUB_SYS: &str = "notify_kafka";
pub const NOTIFY_MQTT_SUB_SYS: &str = "notify_mqtt";
pub const NOTIFY_MYSQL_SUB_SYS: &str = "notify_mysql";
pub const NOTIFY_NATS_SUB_SYS: &str = "notify_nats";
pub const NOTIFY_NSQ_SUB_SYS: &str = "notify_nsq";
pub const NOTIFY_ES_SUB_SYS: &str = "notify_elasticsearch";
pub const NOTIFY_AMQP_SUB_SYS: &str = "notify_amqp";
pub const NOTIFY_POSTGRES_SUB_SYS: &str = "notify_postgres";
pub const NOTIFY_REDIS_SUB_SYS: &str = "notify_redis";
pub const NOTIFY_WEBHOOK_SUB_SYS: &str = "notify_webhook";
// Add new constants here if you add new fields to config.

lazy_static! {
    static ref SUB_SYSTEMS: StringSet = StringSet::from_slice(&[
        CREDENTIALS_SUB_SYS,
        POLICY_OPA_SUB_SYS,
        IDENTITY_OPEN_ID_SUB_SYS,
        IDENTITY_LDAP_SUB_SYS,
        CACHE_SUB_SYS,
        REGION_SUB_SYS,
        ETCD_SUB_SYS,
        STORAGE_CLASS_SUB_SYS,
        API_SUB_SYS,
        COMPRESSION_SUB_SYS,
        KMS_VAULT_SUB_SYS,
        KMS_KES_SUB_SYS,
        LOGGER_WEBHOOK_SUB_SYS,
        AUDIT_WEBHOOK_SUB_SYS,
        HEAL_SUB_SYS,
        SCANNER_SUB_SYS,
        NOTIFY_KAFKA_SUB_SYS,
        NOTIFY_MQTT_SUB_SYS,
        NOTIFY_MYSQL_SUB_SYS,
        NOTIFY_NATS_SUB_SYS,
        NOTIFY_NSQ_SUB_SYS,
        NOTIFY_ES_SUB_SYS,
        NOTIFY_AMQP_SUB_SYS,
        NOTIFY_POSTGRES_SUB_SYS,
        NOTIFY_REDIS_SUB_SYS,
        NOTIFY_WEBHOOK_SUB_SYS,
    ]);
    static ref SUB_SYSTEMS_DYNAMIC: StringSet = StringSet::from_slice(&[
        API_SUB_SYS,
        COMPRESSION_SUB_SYS,
        SCANNER_SUB_SYS,
        HEAL_SUB_SYS,
    ]);
    static ref SUB_SYSTEMS_SINGLE_TARGETS: StringSet = StringSet::from_slice(&[
        CREDENTIALS_SUB_SYS,
        REGION_SUB_SYS,
        ETCD_SUB_SYS,
        CACHE_SUB_SYS,
        API_SUB_SYS,
        STORAGE_CLASS_SUB_SYS,
        COMPRESSION_SUB_SYS,
        KMS_VAULT_SUB_SYS,
        KMS_KES_SUB_SYS,
        POLICY_OPA_SUB_SYS,
        IDENTITY_LDAP_SUB_SYS,
        IDENTITY_OPEN_ID_SUB_SYS,
        HEAL_SUB_SYS,
        SCANNER_SUB_SYS,
    ]);
}

// Constant separators
pub const SUB_SYSTEM_SEPARATOR: &str = ":";
pub const KV_SEPARATOR: &str = "=";
pub const KV_COMMENT: &str = "#";
pub const KV_SPACE_SEPARATOR: &str = " ";
pub const KV_NEWLINE: &str = "\n";
pub const KV_DOUBLE_QUOTE: &str = "\"";
pub const KV_SINGLE_QUOTE: &str = "'";

pub const DEFAULT: &str = "_";

// Env prefix used for all envs in Hulk
pub const ENV_PREFIX: &str = "HULK_";
pub const ENV_WORD_DELIMITER: &str = "_";

lazy_static! {
    // Default kvs for all sub-systems
    pub static ref DEFAULT_KVS: Arc<RwLock<HashMap<String, KVS>>> = Arc::new(RwLock::new(HashMap::new()));

    pub static ref HELP_SUB_SYS_MAP: Arc<RwLock<HelpKVS>> = Arc::new(RwLock::new(HelpKVS::default()));
}

// Register default kvs. Should be called only once.
pub fn register_default_kvs(kvs_map: HashMap<String, KVS>) {
    let mut kvs = DEFAULT_KVS.write().unwrap();
    *kvs = kvs_map;
}

pub fn register_help_sub_sys(help_kvs_map: HelpKVS) {
    let mut kvs = HELP_SUB_SYS_MAP.write().unwrap();
    *kvs = help_kvs_map;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KV {
    pub key: String,
    pub value: String,
}

#[derive(Default, Clone)]
pub struct KVS(pub Vec<KV>);

impl KVS {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, KV> {
        self.0.iter()
    }

    pub fn keys(&self) -> Vec<&str> {
        let mut keys = Vec::with_capacity(self.0.len());
        let mut found_comment = false;
        for kv in &self.0 {
            if kv.key == COMMON_KEY {
                found_comment = true;
            }
            keys.push(kv.key.as_str());
        }
        // Comment Key not found, add it explicitly.
        if !found_comment {
            keys.push(COMMON_KEY);
        }
        keys
    }

    // Sets a key value pair.
    pub fn set(&mut self, key: String, value: String) {
        match self.0.iter_mut().find(|kv| kv.key == key) {
            Some(kv) => {
                kv.value = value;
            }
            None => self.0.push(KV { key, value }),
        }
    }

    pub fn get(&self, key: &str) -> &str {
        self.lookup(key).unwrap_or("")
    }

    pub fn delete(&mut self, key: &str) {
        if let Some(i) = self.0.iter().position(|kv| kv.key == key) {
            self.0.remove(i);
        }
    }

    pub fn lookup(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|&kv| kv.key == key)
            .map(|kv| kv.value.as_str())
    }
}

impl ToString for KVS {
    fn to_string(&self) -> String {
        let mut s = String::new();
        for kv in &self.0 {
            // Do not need to print if state is on
            if kv.key == ENABLE_KEY && kv.value == ENABLE_ON {
                continue;
            }
            s.push_str(&kv.key);
            s.push_str(KV_SEPARATOR);
            let spc = kv.value.contains(char::is_whitespace);
            if spc {
                s.push_str(KV_DOUBLE_QUOTE);
            }
            s.push_str(&kv.value);
            if spc {
                s.push_str(KV_DOUBLE_QUOTE);
            }
            s.push_str(KV_SPACE_SEPARATOR);
        }
        s
    }
}

// Config structure at server.
struct Config(HashMap<String, HashMap<String, KVS>>);

impl Config {
    pub fn del_from<T: std::io::Read>(&mut self, r: T) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn set_kvs(&mut self, s: &str, default_kvs: &HashMap<String, KVS>) -> anyhow::Result<bool> {
        if s.is_empty() {
            bail!("input arguments cannot be empty");
        }
        let inputs: Vec<&str> = s.splitn(2, KV_SPACE_SEPARATOR).collect();
        if inputs.len() <= 1 {
            bail!("invalid number of arguments '{}'", s);
        }
        let sub_system_value: Vec<&str> = inputs[0].splitn(2, SUB_SYSTEM_SEPARATOR).collect();
        if sub_system_value.is_empty() {
            bail!("invalid number of arguments '{}'", s);
        }

        let sub_sys: &str = &sub_system_value[0].to_string();
        if !SUB_SYSTEMS.contains(sub_sys) {
            bail!("unknown sub-system '{}'", sub_sys);
        }

        if SUB_SYSTEMS_SINGLE_TARGETS.contains(sub_sys) && sub_system_value.len() == 2 {
            bail!("sub-system '{}' only supports single target", sub_sys);
        }
        let dynamic = SUB_SYSTEMS_DYNAMIC.contains(sub_sys);

        let target: &str = &if sub_system_value.len() == 2 {
            sub_system_value[1].to_string()
        } else {
            DEFAULT.to_string()
        };

        let default_kvs = default_kvs
            .get(sub_sys)
            .ok_or(anyhow!("default kvs not found"))?;
        let default_keys = default_kvs.keys();

        let fields = kv_fields(inputs[1], &default_keys[..]);
        if fields.is_empty() {
            bail!("sub-system '{}' cannot have empty keys", sub_sys);
        }

        let mut kvs = KVS::default();
        let mut prev_k = ""; // previous key
        for f in fields {
            let kv: Vec<&str> = f.splitn(2, KV_SEPARATOR).collect();
            if kv.is_empty() {
                continue;
            } else if kv.len() == 1 && !prev_k.is_empty() {
                // Merge previous value and this value.
                let v = [kvs.get(prev_k), sanitize_value(kv[0])].join(KV_SPACE_SEPARATOR);
                // Re-set previous key value.
                kvs.set(prev_k.to_owned(), v);
            } else if kv.len() == 2 {
                prev_k = kv[0]; // remember this key
                kvs.set(prev_k.to_owned(), sanitize_value(kv[1]).to_owned());
            } else {
                bail!("key '{}' must have value", kv[0]);
            }
        }

        // Check if state is required.
        let enable_required = default_kvs.lookup(ENABLE_KEY).is_some();
        if kvs.lookup(ENABLE_KEY).is_none() && enable_required {
            // Implicit state "on" if not specified.
            kvs.set(ENABLE_KEY.to_owned(), ENABLE_ON.to_owned());
        }

        let curr_kvs = self
            .0
            .entry(sub_sys.to_string())
            .or_insert(Default::default()) // insert default
            .entry(target.to_string())
            .and_modify(|kvs| {
                // If any key in default_kvs is not found, insert its default kv.
                for kv in default_kvs.iter() {
                    if kvs.lookup(&kv.key).is_none() {
                        kvs.set(kv.key.to_owned(), kv.value.to_owned());
                    }
                }
            })
            .or_insert(default_kvs.clone()); // if not found, insert default_kvs

        for kv in &kvs.0 {
            if kv.key == COMMON_KEY {
                // Skip comment and add it later.
                continue;
            }
            curr_kvs.set(kv.key.to_owned(), kv.value.to_owned());
        }

        if let Some(v) = kvs.lookup(COMMON_KEY) {
            curr_kvs.set(COMMON_KEY.to_owned(), v.to_owned());
        }

        let help_kvs = HELP_SUB_SYS_MAP.read().unwrap();
        for hkv in help_kvs.iter() {
            let enabled = if enable_required {
                curr_kvs.get(ENABLE_KEY) == ENABLE_ON
            } else {
                // When enable arg is not required,
                // then it is implicit on for the sub-system.
                true
            };
            if !hkv.optional && enabled {
                if let Some(v) = curr_kvs.lookup(&hkv.key) {
                    if !v.is_empty() {
                        continue;
                    }
                }
                // Return error only if the
                // key is enabled, for state=off
                // let it be empty.
                bail!(
                    "'{}' is not optional for '{}' sub-system, please check '{}' documentation",
                    hkv.key,
                    sub_sys,
                    sub_sys
                );
            }
        }

        Ok(dynamic)
    }
}

pub fn kv_fields<'a>(input: &'a str, keys: &[&str]) -> Vec<&'a str> {
    let mut value_indexes: Vec<usize> = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(i) = input.find(&((*key).to_owned() + KV_SEPARATOR)) {
            value_indexes.push(i);
        }
    }

    value_indexes.sort_unstable();
    value_indexes
        .iter()
        .enumerate()
        .map(|(i, index)| {
            let mut j = i + 1;
            if j > value_indexes.len() {
                j = value_indexes.len();
            }
            let s = &input[*index..value_indexes[j]];
            s.trim()
        })
        .collect()
}

// Trim off whitespaces, single or double quotes, creeping into the values.
fn sanitize_value(v: &str) -> &str {
    let quotes = KV_DOUBLE_QUOTE
        .chars()
        .chain(KV_SINGLE_QUOTE.chars())
        .collect::<Vec<char>>();
    v.trim().trim_matches(&quotes[..])
}
