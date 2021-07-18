use lazy_static::lazy_static;

use super::*;
use crate::config::{self, HelpKV, HelpKVS};

lazy_static! {
    pub static ref HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: EXTENSIONS.to_owned(),
            description: r#"comma separated file extensions e.g. ".txt,.log,.csv""#.to_owned(),
            optional: true,
            typ: "csv".to_owned(),
            ..Default::default()
        },
        HelpKV {
            key: MIME_TYPES.to_owned(),
            description: r#"comma separated wildcard mime-types e.g. "text/*,application/json,application/xml""#.to_owned(),
            optional: true,
            typ: "csv".to_owned(),
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
