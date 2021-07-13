use serde::{Deserialize, Serialize};
use strum::Display;

// Represents status of a versioned delete or permanent delete w.r.t bucket replication.
#[derive(Serialize, Deserialize, Display)]
pub enum VersionPurgeStatus {
    #[serde(rename = "PENDING")]
    #[strum(serialize = "PENDING")]
    Pending,
    #[serde(rename = "COMPLETE")]
    #[strum(serialize = "COMPLETE")]
    Complete,
    #[serde(rename = "FAILED")]
    #[strum(serialize = "FAILED")]
    Failed,
}
