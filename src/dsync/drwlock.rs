use std::sync::Arc;
use std::time::SystemTime;

use futures::TryFutureExt;
use log::trace;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::{timeout, timeout_at, Duration, Instant};

use super::*;
use crate::dsync::Dsync;

// Tolerance limit to wait for lock acquisition before.
const DRW_MUTEX_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(1);

// Timeout for the refresh call
const DRW_MUTEX_REFRESH_CALL_TIMEOUT: Duration = Duration::from_secs(5);

// Timeout for the unlock call
const DRW_MUTEX_UNLOCK_CALL_TIMEOUT: Duration = Duration::from_secs(30);

// The interval between two refresh calls
const DRW_MUTEX_REFRESH_INTERVAL: Duration = Duration::from_secs(10);

const DRW_MUTEX_INFINITE: i64 = i64::MAX;

// A distributed mutual exclusion lock.
pub struct DRWLock<L: NetLocker, D: Dsync<L>> {
    pub names: Vec<String>,
    dsync: D,
    write_locks: Vec<String>, // Array of nodes that granted a write lock
    readers_locks: Vec<Vec<String>>, // Array of array of nodes that granted reader locks
    _phantom: std::marker::PhantomData<L>,
}

#[derive(Debug)]
pub struct Options {
    pub timeout: Duration,
}

// Represents a structure of a granted lock.
pub struct Granted {
    index: usize,
    lock_uid: String, // Locked if set with UID string, unlocked if empty
}

impl<L: NetLocker, D: Dsync<L>> DRWLock<L, D> {
    pub fn new(dsync: D, names: Vec<String>) -> DRWLock<L, D> {
        let (lockers, _) = dsync.get_lockers();
        DRWLock {
            names,
            dsync,
            write_locks: vec!["".to_owned(); lockers.len()],
            readers_locks: Default::default(),
            _phantom: Default::default(),
        }
    }

    pub async fn try_lock(&mut self, id: &str, source: &str, opts: Options) -> bool {
        todo!()
    }

    pub async fn rlock(&mut self, id: &str, source: &str) {
        todo!()
    }

    pub async fn try_rlock(&mut self, id: &str, source: &str, opts: Options) -> bool {
        todo!()
    }

    async fn lock_blocking(
        &mut self,
        id: &str,
        source: &str,
        is_read_lock: bool,
        opts: Options,
    ) -> bool {
        let lockers = self.dsync.get_lockers();

        let mut rng = StdRng::seed_from_u64(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );

        trace!(
            "lock_blocking {}/{} for {:?}: lock type {}, additional opts: {:?}",
            id,
            source,
            self.names,
            if is_read_lock { "Read" } else { "Write" },
            opts
        );

        true
    }
}

impl Granted {
    fn is_locked(&self) -> bool {
        is_locked(&self.lock_uid)
    }
}

fn is_locked(uid: &str) -> bool {
    !uid.is_empty()
}

async fn lock<L: NetLocker + Send + Sync + 'static, D: Dsync<L>>(
    dsync: &D,
    locks: &mut [String],
    id: &str,
    source: &str,
    is_read_lock: bool,
    tolerance: u8,
    quorum: u8,
    lock_names: &Vec<String>,
) -> bool {
    for l in locks.iter_mut() {
        l.clear();
    }

    let (lockers, owner) = dsync.get_lockers();

    let (tx, mut rx) = channel(lockers.len());

    let mut handles = Vec::new();
    for (index, locker) in lockers.iter().enumerate() {
        // Broadcast lock request to all nodes
        let id = id.to_owned();
        let source = source.to_owned();
        let owner = owner.clone();
        let lock_names = lock_names.clone();
        let tx = tx.clone();
        let locker = locker.clone();
        handles.push(tokio::spawn(async move {
            let mut locker = locker.write().await;

            let mut g = Granted {
                index,
                lock_uid: Default::default(),
            };

            let args = LockArgs {
                uid: id,
                resources: lock_names,
                source,
                owner,
                quorum,
            };

            let locked = if is_read_lock {
                locker.rlock(&args).await
            } else {
                locker.lock(&args).await
            };
            match locked {
                Ok(locked) => {
                    if locked {
                        g.lock_uid = args.uid;
                    }
                }
                Err(err) => {
                    trace!(
                        "dsync: Unable to call {} failed with {} for {:?} at {}\n",
                        if is_read_lock { "rlock" } else { "lock" },
                        err,
                        args,
                        locker.to_string()
                    );
                }
            }
            tx.send(g).await;
        }));
    }

    // Wait until we have either
    // a) received all lock responses
    // b) received too many 'non-'locks for quorum to be still possible
    // c) timed out
    let mut locks_failed = 0;
    // Combined timeout for the lock attempt.
    let deadline = Instant::now() + DRW_MUTEX_ACQUIRE_TIMEOUT;
    // Loop until we acquired all locks
    for i in 0..lockers.len() {
        let grant = timeout_at(deadline, rx.recv()).await;
        match grant {
            Ok(grant) => {
                if let Some(grant) = grant {
                    if grant.is_locked() {
                        // Mark that this node has acquired the lock
                        locks[grant.index] = grant.lock_uid;
                    } else {
                        locks_failed += 1;
                        if locks_failed > tolerance {
                            // We know that we are not going to get the lock anymore,
                            // so exit out and release any locks that did get acquired
                            break;
                        }
                    }
                } else {
                    // Channel is closed
                    break;
                }
            }
            Err(_) => {
                // Captured timeout, locks as failed or took too long
                locks_failed += 1;
                if locks_failed > tolerance {
                    // We know that we are not going to get the lock anymore,
                    // so exit out and release any locks that did get acquired
                    break;
                }
            }
        }
    }

    let r = futures::future::join_all(handles).await;
    rx.close();
    true
}

// Determines whether we have locked the required quorum of underlying locks or not.
fn check_quorum_locked(locks: &[String], quorum: u8) -> bool {
    locks.iter().filter(|&uid| is_locked(uid)).count() > quorum as usize
}
