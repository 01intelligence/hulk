use serde::{Deserialize, Serialize};

use crate::bitrot;

#[derive(Serialize, Deserialize)]
pub struct ObjectPartInfo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub etag: String,
    pub number: isize,
    pub size: i64,
    #[serde(rename = "actualSize")]
    pub actual_size: i64,
}

pub struct ErasureInfo {
    pub algorithm: String,
    pub data_blocks: usize,
    pub parity_blocks: usize,
    pub block_size: u64,
    pub index: usize,
    pub distribution: Vec<u8>,
    pub checksums: Vec<ChecksumInfo>,
}

pub struct ChecksumInfo {
    pub part_number: usize,
    pub algorithm: bitrot::BitrotAlgorithm,
    pub hash: Vec<u8>,
}
