use lazy_static::lazy_static;

use super::*;
use crate::config::{self, HelpKV, HelpKVS};

lazy_static! {
    pub static ref HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: DRIVES.to_owned(),
            description: r#"comma separated mountpoints e.g. "/optane1,/optane2""#.to_owned(),
            typ: "csv".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: EXPIRY.to_owned(),
            description: r#"cache expiry duration in days e.g. "90""#.to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: QUOTA.to_owned(),
            description: r#"limit cache drive usage in percentage e.g. "90""#.to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: EXCLUDE.to_owned(),
            description: r#"exclude cache for following patterns e.g. "bucket/*.tmp,*.exe""#.to_owned(),
            optional: true,
            typ: "csv".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: AFTER.to_owned(),
            description: "minimum number of access before caching an object".to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: WATERMARK_LOW.to_owned(),
            description: "% of cache use at which to stop cache eviction".to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: WATERMARK_HIGH.to_owned(),
            description: "% of cache use at which to start cache eviction".to_owned(),
            optional: true,
            typ: "number".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: RANGE.to_owned(),
            description: r#"set to "on" or "off" caching of independent range requests per object, defaults to "on""#.to_owned(),
            optional: true,
            typ: "string".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: COMMIT.to_owned(),
            description: r#"set to control cache commit behavior, defaults to "writethrough""#.to_owned(),
            optional: true,
            typ: "string".to_owned(),
            ..Default::default()
            },
        HelpKV {
            key: COMMENT_KEY.to_owned(),
            description: DEFAULT_COMMENT.to_owned(),
            optional: true,
            typ: "sentence".to_owned(),
            ..Default::default()
        },
    ]);
}
