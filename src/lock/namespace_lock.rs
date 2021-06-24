use async_trait::async_trait;

use super::*;
use crate::dsync::{DRWLock, Dsync, NetLocker};

#[async_trait]
pub trait RWLocker {
    async fn lock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()>;
    async fn unlock(&mut self);
    async fn rlock(&mut self, timeout: DynamicTimeout) -> anyhow::Result<()>;
    async fn runlock(&mut self);
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
