use std::collections::HashMap;

use hulk::{config, globals, logger};

use super::*;

pub fn init_help() {
    use config::*;
    let mut kvs: HashMap<String, KVS> = maplit::hashmap! {
        ETCD_SUB_SYS.to_owned() => etcd::DEFAULT_KVS.clone(),
        CACHE_SUB_SYS.to_owned() => cache::DEFAULT_KVS.clone(),
        COMPRESSION_SUB_SYS.to_owned() => compress::DEFAULT_KVS.clone(),
        IDENTITY_OPEN_ID_SUB_SYS.to_owned() => openid::DEFAULT_KVS.clone(),
        REGION_SUB_SYS.to_owned() => DEFAULT_REGION_KVS.clone(),
        API_SUB_SYS.to_owned() => api::DEFAULT_KVS.clone(),
        CREDENTIALS_SUB_SYS.to_owned() => DEFAULT_CREDENTIAL_KVS.clone(),
        LOGGER_WEBHOOK_SUB_SYS.to_owned() => logger::DEFAULT_KVS.clone(),
        AUDIT_WEBHOOK_SUB_SYS.to_owned() => logger::DEFAULT_AUDIT_KVS.clone(),
        HEAL_SUB_SYS.to_owned() => heal::DEFAULT_KVS.clone(),
        SCANNER_SUB_SYS.to_owned() => scanner::DEFAULT_KVS.clone(),
    };
    for (k, v) in notify::DEFAULT_KVS.iter() {
        kvs.insert(k.to_owned(), v.clone());
    }
    if *globals::GLOBAL_IS_ERASURE.lock().unwrap() {
        kvs.insert(
            STORAGE_CLASS_SUB_SYS.to_owned(),
            storageclass::DEFAULT_KVS.clone(),
        );
    }
    register_default_kvs(kvs);
}
