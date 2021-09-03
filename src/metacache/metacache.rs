use super::*;
use crate::utils::{self, DateTimeExt, Duration};

pub enum ScanStatus {
    None,
    Started,
    Success,
    Error,
}

// Time in which the initiator of a scan must have reported back.
pub const METACACHE_MAX_RUNNING_AGE: Duration = crate::utils::minutes(1);

// The number of file/directory entries to have in each block.
pub const METACACHE_BLOCK_SIZE: usize = 5000;

// Controls whether prefixes on dirty paths are always shared.
// This will make `test/a` and `test/b` share listings if they are concurrent.
// Enabling this will make cache sharing more likely and cause less IO,
// but may cause additional latency to some calls.
pub const METACACHE_SHARE_PREFIX: bool = false;

/// Represents a tracked cache entry.
pub struct MetaCache {
    pub id: String,
    pub bucket: String,
    pub root: String,
    pub recursive: bool,
    pub filter: String,
    pub status: ScanStatus,
    pub file_not_found: bool,
    pub error: Option<String>,
    pub started: utils::DateTime,
    pub ended: utils::DateTime,
    pub last_update: utils::DateTime,
    pub last_handout: utils::DateTime,
    pub started_cycle: u64,
    pub ended_cycle: u64,
    pub data_version: u8,
}

impl MetaCache {
    fn finished(&self) -> bool {
        !self.ended.is_zero()
    }

    fn matches(&self, options: &ListPathOptions, extend: Duration) -> bool {
        todo!()
    }

    fn worth_keeping(&self, current_cycle: u64) -> bool {
        todo!()
    }

    fn can_be_replaced_by(&self, other: &MetaCache) -> bool {
        todo!()
    }

    fn update(&mut self, update: &MetaCache) {
        todo!()
    }

    async fn delete(&mut self) {
        todo!()
    }
}
