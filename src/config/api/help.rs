use lazy_static::lazy_static;

use super::*;
use crate::config::{self, HelpKV, HelpKVS};

lazy_static! {
    pub static ref HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: API_REQUESTS_MAX.to_owned(),
            description: r#"set the maximum number of concurrent requests, e.g. "1600""#.to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: API_REQUESTS_DEADLINE.to_owned(),
            description: r#"set the deadline for API requests waiting to be processed e.g. "1m""#.to_owned(),
            optional: true,
            typ: "duration".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: API_CORS_ALLOW_ORIGIN.to_owned(),
            description: r#"set comma separated list of origins allowed for CORS requests e.g. "https://example1.com,https://example2.com""#.to_owned(),
            optional: true,
            typ: "csv".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: API_REMOTE_TRANSPORT_DEADLINE.to_owned(),
            description: r#"set the deadline for API requests on remote transports while proxying between federated instances e.g. "2h""#.to_owned(),
            optional: true,
            typ: "duration".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: API_REPLICATION_WORKERS.to_owned(),
            description: r#"set the number of replication workers, defaults to 100"#.to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: API_REPLICATION_FAILED_WORKERS.to_owned(),
            description: r#"set the number of replication workers for recently failed replicas, defaults to 4"#.to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
    ]);
}
