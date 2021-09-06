use serde::{Deserialize, Serialize};

use crate::{bitrot, utils};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObjectPartInfo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub etag: String,
    pub number: usize,
    pub size: u64,
    #[serde(rename = "actualSize")]
    pub actual_size: i64,
}

/// Holds erasure-coding and bitrot related information.
#[derive(Clone, Debug)]
pub struct ErasureInfo {
    /// Erasure-coding algorithm.
    pub algorithm: String,
    /// Number of data blocks for erasure-coding.
    pub data_blocks: usize,
    /// Number of parity blocks for erasure-coding.
    pub parity_blocks: usize,
    /// Size of one erasure-coding block.
    pub block_size: u64,
    /// Index of the current disk.
    pub index: usize,
    /// Distribution of the data and parity blocks.
    pub distribution: Vec<u8>,
    /// All bitrot checksums of all erasure-coding blocks.
    pub checksums: Vec<ChecksumInfo>,
}

/// Checksum of individual scattered part.
#[derive(Clone, Debug)]
pub struct ChecksumInfo {
    pub part_number: usize,
    pub algorithm: bitrot::BitrotAlgorithm,
    pub hash: Vec<u8>,
}

impl ErasureInfo {
    pub fn add_checksum_info(&mut self, ck_info: ChecksumInfo) {
        for checksum in &mut self.checksums {
            if checksum.part_number == ck_info.part_number {
                *checksum = ck_info;
                return;
            }
        }
        self.checksums.push(ck_info);
    }

    pub fn get_checksum_info(&self, part_number: usize) -> Option<&ChecksumInfo> {
        for checksum in &self.checksums {
            if checksum.part_number == part_number {
                return Some(checksum);
            }
        }
        None
    }

    pub fn shard_file_size(&self, total_length: u64) -> u64 {
        assert!(total_length > 0);
        let shards_num = total_length / self.block_size;
        let last_block_size = total_length % self.block_size;
        let last_shard_size = utils::ceil_frac(last_block_size as u64, self.data_blocks as u64);
        shards_num * self.shard_size() + last_shard_size
    }

    pub fn shard_size(&self) -> u64 {
        utils::ceil_frac(self.block_size as u64, self.data_blocks as u64)
    }
}
