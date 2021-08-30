use serde::{Deserialize, Serialize};

use crate::utils;

#[derive(Serialize, Deserialize)]
pub struct ConfigHistoryEntry {
    #[serde(rename = "restoreId")]
    pub restore_id: String,
    #[serde(rename = "createTime")]
    pub create_time: utils::DateTime,
    pub data: String,
}
