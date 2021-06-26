use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use log::error;
use tokio::sync::Mutex;
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

#[derive(Default)]
struct NamespaceLock {
    refs: i32,
    lock: Arc<Mutex<TimedRWLock>>,
}

struct NamespaceLockMap {
    // Indicates if namespace is part of a distributed setup.
    is_dist_erasure: bool,
    lock_map: Mutex<HashMap<String, NamespaceLock>>,
}

impl NamespaceLockMap {
    async fn lock(
        &self,
        volume: &str,
        path: &str,
        lock_source: &str,
        ops_id: &str,
        read_lock: bool,
        timeout: Duration,
    ) {
        let resource = crate::object::path_join(&[volume, path]);
        let lock;
        {
            let mut lock_map = self.lock_map.lock().await;
            let ns_lock = lock_map
                .entry(resource.clone())
                .or_insert(NamespaceLock::default());
            ns_lock.refs += 1;
            lock = ns_lock.lock.clone();
            // Drop MutexGuard
        }

        let locked = if read_lock {
            lock.lock().await.rlock(timeout).await
        } else {
            lock.lock().await.lock(timeout).await
        };

        if !locked {
            // Decrement ref count since we failed to get the lock.
            let mut lock_map = self.lock_map.lock().await;
            let ns_lock = lock_map.get_mut(&resource).unwrap();
            ns_lock.refs -= 1;
            if ns_lock.refs < 0 {
                error!("resource reference count was lower than 0");
            }
            if ns_lock.refs == 0 {
                // Remove from the map if there are no more references.
                lock_map.remove(&resource);
            }
            // Drop MutexGuard
        }
    }

    async fn unlock(&self, volume: &str, path: &str, read_lock: bool) {
        let resource = crate::object::path_join(&[volume, path]);
        let mut lock_map = self.lock_map.lock().await;
        let ns_lock = match lock_map.get_mut(&resource) {
            None => {
                return;
            }
            Some(ns_lock) => ns_lock,
        };
        if read_lock {
            ns_lock.lock.lock().await.runlock();
        } else {
            ns_lock.lock.lock().await.unlock();
        }
        ns_lock.refs -= 1;
        if ns_lock.refs < 0 {
            error!("resource reference count was lower than 0");
        }
        if ns_lock.refs == 0 {
            // Remove from the map if there are no more references.
            lock_map.remove(&resource);
        }
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
