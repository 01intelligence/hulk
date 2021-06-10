use lazy_static::lazy_static;

use super::*;
use crate::config::{self, HelpKV, HelpKVS};

lazy_static! {
    pub static ref HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: CLASS_STANDARD.to_owned(),
            description: r#"set the parity count for default standard storage class e.g. "EC:4""#
                .to_owned(),
            optional: true,
            typ: "string".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: CLASS_RRS.to_owned(),
            description: r#"set the parity count for reduced redundancy storage class e.g. "EC:2""#
                .to_owned(),
            optional: true,
            typ: "string".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: CLASS_DMA.to_owned(),
            description:
                r#"enable O_DIRECT for both read and write, defaults to "write" e.g. "read+write""#
                    .to_owned(),
            optional: true,
            typ: "string".to_owned(),
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
