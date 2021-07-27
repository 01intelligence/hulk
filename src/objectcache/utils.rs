use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Serialize, Deserialize, Display)]
pub enum CacheStatus {
    #[serde(rename = "HIT")]
    #[strum(serialize = "HIT")]
    Hit,
    #[serde(rename = "MISS")]
    #[strum(serialize = "MISS")]
    Miss,
}
