use super::*;
use crate::storage::StorageApi;
use crate::utils;

pub struct ListPathOptions {
    pub id: String,
    pub bucket: String,
    pub base_dir: String,
    pub prefix: String,
    pub filter_prefix: String,
    pub marker: String,
    pub limit: usize,
    pub ask_disks: usize,
    pub include_deleted: bool,
    pub recursive: bool,
    pub separator: String,
    pub create: bool,
    pub current_cycle: u64,
    pub oldest_cycle: u64,
    pub include_directories: bool,
    pub transient: bool,
    discard_result: bool,
}

impl ListPathOptions {
    pub fn new_metacache(&self) -> MetaCache {
        MetaCache {
            id: self.id.clone(),
            bucket: self.bucket.clone(),
            root: self.base_dir.clone(),
            recursive: self.recursive,
            filter: self.filter_prefix.clone(),
            status: ScanStatus::Started,
            file_not_found: false,
            error: None,
            started: utils::now(),
            ended: Default::default(),
            last_update: utils::now(),
            last_handout: utils::now(),
            started_cycle: self.current_cycle,
            ended_cycle: 0,
            data_version: METACACHE_STREAM_VERSION,
        }
    }
}

pub struct ListPathRawOptions {
    disks: Vec<StorageApi>,
    bucket: String,
    path: String,
    recursive: bool,
    filter_prefix: String,
    forward_to: String,
    min_disks: usize,
    report_not_found: bool,
    agreed: Box<dyn Fn(MetaCacheEntry)>,
    partial: Box<dyn Fn(MetaCacheEntry, usize, Vec<Option<anyhow::Error>>)>,
    finished: Box<dyn Fn(Vec<Option<anyhow::Error>>)>,
}
