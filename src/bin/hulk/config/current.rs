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
    if globals::GLOBALS.is_erasure.get() {
        kvs.insert(
            STORAGE_CLASS_SUB_SYS.to_owned(),
            storageclass::DEFAULT_KVS.clone(),
        );
    }
    register_default_kvs(kvs);

    let mut help_sub_sys = HelpKVS(vec![
        HelpKV {
            key: REGION_SUB_SYS.to_string(),
            description: "label the location of the server".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: CACHE_SUB_SYS.to_string(),
            description: "add caching storage tier".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: COMPRESSION_SUB_SYS.to_string(),
            description: "enable server side compression of objects".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: ETCD_SUB_SYS.to_string(),
            description: "federate multiple clusters for IAM and Bucket DNS".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: IDENTITY_OPEN_ID_SUB_SYS.to_string(),
            description: "enable OpenID SSO support".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: KMS_KES_SUB_SYS.to_string(),
            description: "enable external Hulk key encryption service".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: API_SUB_SYS.to_string(),
            description: "manage global HTTP API call specific features, such as throttling, authentication types, etc.".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: HEAL_SUB_SYS.to_string(),
            description: "manage object healing frequency and bitrot verification checks".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: SCANNER_SUB_SYS.to_string(),
            description: "manage namespace scanning for usage calculation, lifecycle, healing and more".to_string(),
            ..Default::default()
        },
        HelpKV {
            key: LOGGER_WEBHOOK_SUB_SYS.to_string(),
            description: "send server logs to webhook endpoints".to_string(),
            multiple_targets: true,
            ..Default::default()
        },
        HelpKV {
            key: AUDIT_WEBHOOK_SUB_SYS.to_string(),
            description: "send audit logs to webhook endpoints".to_string(),
            multiple_targets: true,
            ..Default::default()
        },
        HelpKV {
            key: NOTIFY_WEBHOOK_SUB_SYS.to_string(),
            description: "publish bucket notifications to webhook endpoints".to_string(),
            multiple_targets: true,
            ..Default::default()
        },
        HelpKV {
            key: NOTIFY_NATS_SUB_SYS.to_string(),
            description: "publish bucket notifications to NATS endpoints".to_string(),
            multiple_targets: true,
            ..Default::default()
        },
        HelpKV {
            key: NOTIFY_REDIS_SUB_SYS.to_string(),
            description: "publish bucket notifications to Redis endpoints".to_string(),
            multiple_targets: true,
            ..Default::default()
        },
        HelpKV {
            key: NOTIFY_MYSQL_SUB_SYS.to_string(),
            description: "publish bucket notifications to MySQL endpoints".to_string(),
            multiple_targets: true,
            ..Default::default()
        },
        HelpKV {
            key: NOTIFY_ES_SUB_SYS.to_string(),
            description: "publish bucket notifications to Elasticsearch endpoints".to_string(),
            multiple_targets: true,
            ..Default::default()
        },
    ]);
    if globals::GLOBALS.is_erasure.get() {
        help_sub_sys.0.insert(
            1,
            HelpKV {
                key: STORAGE_CLASS_SUB_SYS.to_string(),
                description: "define object level redundancy".to_string(),
                ..Default::default()
            },
        )
    }
    let help_map: HashMap<String, HelpKVS> = maplit::hashmap! {
        "".to_owned() => help_sub_sys,
        REGION_SUB_SYS.to_owned() => REGION_HELP.clone(),
        API_SUB_SYS.to_owned() => api::HELP.clone(),
        STORAGE_CLASS_SUB_SYS.to_owned() => storageclass::HELP.clone(),
        ETCD_SUB_SYS.to_owned() => etcd::HELP.clone(),
        CACHE_SUB_SYS.to_owned() => cache::HELP.clone(),
        COMPRESSION_SUB_SYS.to_owned() => compress::HELP.clone(),
        HEAL_SUB_SYS.to_owned() => heal::HELP.clone(),
        SCANNER_SUB_SYS.to_owned() => scanner::HELP.clone(),
        IDENTITY_OPEN_ID_SUB_SYS.to_owned() => openid::HELP.clone(),
        /*LOGGER_WEBHOOK_SUB_SYS.to_owned() => logger::HELP.clone(),
        AUDIT_WEBHOOK_SUB_SYS.to_owned() => logger::HELP_AUDIT.clone(),
        NOTIFY_WEBHOOK_SUB_SYS.to_owned() => notify::HELP_WEBHOOK.clone(),
        NOTIFY_NATS_SUB_SYS.to_owned() => notify::HELP_NATS.clone(),
        NOTIFY_REDIS_SUB_SYS.to_owned() => notify::HELP_REDIS.clone(),
        NOTIFY_MYSQL_SUB_SYS.to_owned() => notify::HELP_MYSQL.clone(),
        NOTIFY_ES_SUB_SYS.to_owned() => notify::HELP_ES.clone(),*/
    };
    register_help_sub_sys(help_map);
}
