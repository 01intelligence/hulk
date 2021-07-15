use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ConfigHistoryEntry {
    #[serde(rename = "restoreId")]
    pub restore_id: String,
    #[serde(rename = "createTime")]
    pub create_time: DateTime<Utc>,
    pub data: String,
}
