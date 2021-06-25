use std::collections::HashMap;

use async_trait::async_trait;
use tokio::time::{timeout_at, Duration, Instant};

use super::*;
use crate::dsync::{DRWLock, Dsync, NetLocker};

#[async_trait]
pub trait RWLocker {
    async fn lock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()>;
    async fn unlock(&mut self);
    async fn rlock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()>;
    async fn runlock(&mut self);
}

struct NamespaceLock {
    refs: u32,
    lock: TimedRWLock,
}

struct NamespaceLockMap {
    // Indicates if namespace is part of a distributed setup.
    is_dist_erasure: bool,
    lock_map: HashMap<String, NamespaceLock>,
}

impl NamespaceLockMap {
    fn lock(volume: &str, path: &str, lock_source: &str, ops_id: &str, read_lock: bool, timeout: Duration) {
        let resource = crate::object::path_join(&[volume, path]);
    }
}

// Distributed lock instance from dsync.
struct DistLockInstance<L: NetLocker + Send + Sync + 'static, D: Dsync<L> + Send + Sync + 'static> {
    lock: DRWLock<L, D>,
    ops_id: String,
}

#[async_trait]
impl<L: NetLocker + Send + Sync + 'static, D: Dsync<L> + Send + Sync + 'static> RWLocker
    for DistLockInstance<L, D>
{
    async fn lock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()> {
        let lock_source = get_source();
        Ok(())
    }

    async fn unlock(&mut self) {}

    async fn rlock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()> {
        Ok(())
    }

    async fn runlock(&mut self) {}
}

struct LocalLockInstance {
    volume: String,
    paths: Vec<String>,
    ops_id: String,
}

#[async_trait]
impl RWLocker for LocalLockInstance {
    async fn lock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()> {
        let lock_source = get_source();
        let start = Instant::now();
        let read_lock = false;
        for path in &self.paths {}
        Ok(())
    }

    async fn unlock(&mut self) {}

    async fn rlock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()> {
        Ok(())
    }

    async fn runlock(&mut self) {}
}

fn get_source() -> String {
    // TODO: use backtrace
    "".to_owned()
}
