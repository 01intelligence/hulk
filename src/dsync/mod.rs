mod drwlock;

use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
pub use drwlock::*;

// Dsync represents dsync client object which is initialized with
// authenticated clients, used to initiate lock RPC calls.
pub trait Dsync<L: NetLocker> {
    // List of RPC client objects, one per lock server.
    fn get_lockers(&self) -> (Vec<Arc<RwLock<L>>>, String);
}

#[async_trait]
pub trait NetLocker: ToString {
    async fn rlock(&mut self, args: &LockArgs) -> anyhow::Result<bool>;
    async fn lock(&mut self, args: &LockArgs) -> anyhow::Result<bool>;
    async fn runlock(&mut self, args: &LockArgs) -> anyhow::Result<bool>;
    async fn unlock(&mut self, args: &LockArgs) -> anyhow::Result<bool>;
    async fn refresh(&mut self, args: &LockArgs) -> anyhow::Result<bool>;
    async fn force_unlock(&mut self, args: &LockArgs) -> anyhow::Result<bool>;
    // Closes any underlying connection to the service endpoint
    async fn close(&mut self) -> anyhow::Result<()>;
    // Is the underlying connection online? (is always true for any local lockers)
    fn is_online(&self) -> bool;
    // Is the underlying locker local to this server?
    fn is_local(&self) -> bool;
}

// LockArgs is minimal required values for any dsync compatible lock operation.
#[derive(Debug)]
pub struct LockArgs {
    // Unique ID of lock/unlock request.
    uid: String,
    // Resources contains single or multiple entries to be locked/unlocked.
    resources: Vec<String>,
    // Source contains the line number, function and file name of the code
    // on the client node that requested the lock.
    source: String,
    // Owner represents unique ID for this instance, an owner who originally requested
    // the locked resource, useful primarily in figuring our stale locks.
    owner: String,
    // Quorum represents the expected quorum for this lock type.
    quorum: u8,
}
