use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ObjectPartInfo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub etag: String,
    pub number: isize,
    pub size: i64,
    #[serde(rename = "actualSize")]
    pub actual_size: i64,
}
