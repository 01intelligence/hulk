use serde::{Deserialize, Serialize};

use super::*;

// Represents Erasure backend.
const FORMAT_BACKEND_ERASURE: &str = "xl";

// formatErasureV3.Erasure.Version - version '3'.
const FORMAT_ERASURE_VERSION_V3: &str = "3";

// Distributed algorithm used, with N/2 default parity
const FORMAT_ERASURE_VERSION_V3DISTRIBUTION_ALGO_V2: &str = "SIPMOD";

// Distributed algorithm used, with EC:4 default parity
const FORMAT_ERASURE_VERSION_V3DISTRIBUTION_ALGO_V3: &str = "SIPMOD+PARITY";

// Offline disk UUID represents an offline disk.
const OFFLINE_DISK_UUID: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";

#[derive(Serialize, Deserialize)]
pub struct FormatErasureV3 {
    #[serde(flatten)]
    pub meta: FormatMetaV1,
    #[serde(rename = "xl")]
    pub erasure: ErasureV3,
}

#[derive(Serialize, Deserialize)]
pub struct ErasureV3 {
    pub version: String,
    pub this: String,
    pub sets: Vec<Vec<String>>,
    #[serde(rename = "distributionAlgo")]
    pub distribution_algo: String,
}
