use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

mod storageclass;

pub const ENABLE_KEY: &str = "enable";
pub const COMMON_KEY: &str = "comment";
pub const ENABLE_ON: &str = "on";
pub const ENABLE_OFF: &str = "off";
pub const REGION_NAME: &str = "name";
pub const ACCESS_KEY: &str = "access_key";
pub const SECRET_KEY: &str = "secret_key";

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
    static ref DEFAULT_KVS: Arc<RwLock<HashMap<String, KVS>>> = Arc::new(RwLock::new(HashMap::new()));
}

// Register default kvs. Should be called only once.
pub fn register_default_kvs(kvs_map: HashMap<String, KVS>) {
    let mut kvs = DEFAULT_KVS.write().unwrap();
    *kvs = kvs_map;
}

#[derive(Serialize, Deserialize)]
pub struct KV {
    pub key: String,
    pub value: String,
}

pub struct KVS(pub Vec<KV>);

impl KVS {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn keys(&self) -> Vec<String> {
        let mut keys = Vec::with_capacity(self.0.len());
        let mut found_comment = false;
        for kv in &self.0 {
            if kv.key == COMMON_KEY {
                found_comment = true;
            }
            keys.push(kv.key.clone());
        }
        // Comment Key not found, add it explicitly.
        if !found_comment {
            keys.push(COMMON_KEY.into());
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

    pub fn get(&self, key: String) -> String {
        self.lookup(key).unwrap_or("".to_string())
    }

    pub fn delete(&mut self, key: String) {
        if let Some(i) = self.0.iter().position(|kv| kv.key == key) {
            self.0.remove(i);
        }
    }

    pub fn lookup(&self, key: String) -> Option<String> {
        self.0
            .iter()
            .find(|&kv| kv.key == key)
            .map(|kv| kv.value.clone())
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
}
