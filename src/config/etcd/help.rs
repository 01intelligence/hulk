use lazy_static::lazy_static;

use super::*;
use crate::config::{self, HelpKV, HelpKVS};

lazy_static! {
    pub static ref HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: ENDPOINTS.to_owned(),
            description: r#"comma separated list of etcd endpoints e.g. "http://localhost:2379""#
                .to_owned(),
            typ: "csv".to_owned(),
            sensitive: true,
            ..Default::default()
        },
        HelpKV {
            key: PATH_PREFIX.to_owned(),
            description: r#"namespace prefix to isolate tenants e.g. "customer1/""#.to_owned(),
            optional: true,
            typ: "path".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: CORE_DNS_PATH.to_owned(),
            description: r#"shared bucket DNS records, default is "/skydns""#.to_owned(),
            optional: true,
            typ: "path".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: CLIENT_CERT.to_owned(),
            description: "client cert for mTLS authentication".to_owned(),
            optional: true,
            typ: "path".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: CLIENT_CERT_KEY.to_owned(),
            description: "client cert key for mTLS authentication".to_owned(),
            optional: true,
            typ: "path".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: config::COMMON_KEY.to_owned(),
            description: config::DEFAULT_COMMENT.to_owned(),
            optional: true,
            typ: "sentence".to_owned(),
            ..Default::default()
        },
    ]);
}
