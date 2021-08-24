use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::{utils, xl_storage};

pub struct DiskInfo {
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub used_inodes: u64,
    pub free_inodes: u64,
    pub fs_type: String,
    pub root_disk: bool,
    pub healing: bool,
    pub endpoint: String,
    pub mount_path: String,
    pub id: String,
    pub metrics: DiskMetrics,
    pub error: String,
}

pub struct DiskMetrics {
    pub api_latencies: HashMap<String, String>,
    pub api_calls: HashMap<String, u64>,
}

pub struct VolsInfo(pub Vec<VolInfo>);

pub struct VolInfo {
    pub name: String,
    pub created: utils::DateTime,
}

pub struct FilesInfo {
    pub files: Vec<FileInfo>,
    pub is_truncated: bool,
}

pub struct FilesInfoVersions {
    pub files_versions: Vec<FileInfoVersions>,
    pub is_truncated: bool,
}

pub struct FileInfoVersions {
    pub volume: String,
    pub name: String,
    pub is_empty_dir: bool,
    pub latest_mod_time: utils::DateTime,
    pub versions: Vec<FileInfo>,
}

pub struct FileInfo {
    pub volume: String,
    pub name: String,
    pub version_id: String,
    pub is_latest: bool,
    pub deleted: bool,
    pub transition_status: String,
    pub transition_object_name: String,
    pub transition_tier: String,
    pub transition_version_id: String,
    pub expire_restored: bool,
    pub data_dir: String,
    pub mod_time: utils::DateTime,
    pub size: u64,
    pub mode: u32,
    pub metadata: HashMap<String, String>,
    pub parts: Vec<xl_storage::ObjectPartInfo>,
    pub erasure: Option<xl_storage::ErasureInfo>,
    pub mark_deleted: bool,
    pub delete_marker_replication_status: String,
    pub version_purge_status: Option<VersionPurgeStatus>,
    pub data: Vec<u8>,
    pub num_versions: usize,
    pub successor_mod_time: utils::DateTime,
}

pub const VERSION_PURGE_STATUS_KEY: &str = "purgestatus";

// Represents status of a versioned delete or permanent delete w.r.t bucket replication.
#[derive(Serialize, Deserialize, Clone, Copy, Display, EnumString, PartialEq)]
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
