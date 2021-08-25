use crate::utils;

pub struct HealingTracker {
    id: String,
    pool_index: isize,
    set_index: isize,
    disk_index: isize,
    path: String,
    endpoint: String,
    started: utils::DateTime,
    last_update: utils::DateTime,
    objects_healed: u64,
    objects_failed: u64,
    bytes_done: u64,
    bytes_failed: u64,

    bucket: String,
    object: String,

    resume_objects_healed: u64,
    resume_objects_failed: u64,
    resume_bytes_done: u64,
    resume_bytes_failed: u64,

    queued_buckets: Vec<String>,

    healed_buckets: Vec<String>,
}
