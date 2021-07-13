use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Serialize, Deserialize, Display)]
pub enum Status {
    #[serde(rename = "")]
    #[strum(serialize = "")]
    Unspecified,
    #[serde(rename = "PENDING")]
    #[strum(serialize = "PENDING")]
    Pending,
    #[serde(rename = "COMPLETED")]
    #[strum(serialize = "COMPLETED")]
    Completed,
    #[serde(rename = "FAILED")]
    #[strum(serialize = "FAILED")]
    Failed,
    #[serde(rename = "REPLICA")]
    #[strum(serialize = "REPLICA")]
    Replica,
}
