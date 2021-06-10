use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

// Implements help messages for keys
// with value as description of the keys.
#[derive(Serialize, Deserialize, Default)]
pub struct HelpKV {
    pub key: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub description: String,
    pub optional: bool,

    // Indicates if sub-sys supports multiple targets.
    #[serde(rename = "multipleTargets")]
    pub multiple_targets: bool,
}

#[derive(Default)]
pub struct HelpKVS(pub Vec<HelpKV>);

impl HelpKVS {
    pub fn lookup(&self, key: &str) -> Option<&HelpKV> {
        self.0.iter().find(|kv| kv.key == key)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, HelpKV> {
        self.0.iter()
    }
}

pub const DEFAULT_COMMENT: &str = "optionally add a comment to this setting";

lazy_static! {
    pub static ref REGION_HELP: HelpKVS = HelpKVS(vec![
        HelpKV {
            key: super::config::REGION_NAME.to_string(),
            typ: "string".to_string(),
            description: r#"name of the location of the server e.g. "us-west-rack2""#.to_string(),
            optional: true,
            ..Default::default()
        },
        HelpKV {
            key: super::config::COMMON_KEY.to_string(),
            typ: "sentence".to_string(),
            description: DEFAULT_COMMENT.to_string(),
            optional: true,
            ..Default::default()
        },
    ]);
}
