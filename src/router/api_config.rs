use std::sync::{Arc, Mutex};

use crate::config::api;
use crate::utils::Duration;

#[derive(Default)]
pub struct ApiConfig {
    pub requests_deadline: Duration,
    pub cluster_deadline: Duration,
    pub list_quorum: isize,
    pub extend_list_life: Duration,
    pub cors_allow_origins: Vec<String>,
    pub total_drive_count: usize, // total drives per erasure set across pools
    pub replication_workers: usize,
    pub replication_failed_workers: usize,
}

impl ApiConfig {
    pub fn init(&mut self, cfg: &api::Config, set_drive_counts: &[usize]) {
        self.cluster_deadline = if cfg.cluster_deadline != Duration::ZERO {
            cfg.cluster_deadline
        } else {
            Duration::from_secs(10)
        };
        self.cors_allow_origins = cfg.cors_allow_origin.clone();
        self.total_drive_count = set_drive_counts.iter().fold(0, |acc, &e| acc + e);

        // TODO
        if cfg.requests_max == 0 {}

        self.requests_deadline = cfg.requests_deadline;
        self.list_quorum = cfg.get_list_quorum();
        self.extend_list_life = cfg.extend_list_cache_life;
        self.replication_workers = cfg.replication_workers;
        self.replication_failed_workers = cfg.replication_failed_workers;
    }
}
