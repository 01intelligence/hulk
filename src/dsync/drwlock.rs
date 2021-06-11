use std::sync::Arc;
use std::time::SystemTime;

use anyhow::anyhow;
use futures::{FutureExt, TryFutureExt};
use log::trace;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::select;
use tokio::sync::mpsc::channel;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::{timeout, timeout_at, Duration, Instant};
use tokio_util::sync::CancellationToken;

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

const DRW_MUTEX_INFINITE: Duration = Duration::from_nanos(u64::MAX);

const LOCK_RETRY_INTERVAL: Duration = Duration::from_secs(1);

// A distributed mutual exclusion lock.
pub struct DRWLock<L: NetLocker + Send + Sync + 'static, D: Dsync<L>> {
    pub names: Vec<String>,
    dsync: D,
    write_locks: Arc<RwLock<Vec<String>>>, // Array of nodes that granted a write lock
    readers_locks: Arc<RwLock<Vec<Vec<String>>>>, // Array of array of nodes that granted reader locks
    token: CancellationToken,
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

impl<L: NetLocker + Send + Sync + 'static, D: Dsync<L>> DRWLock<L, D> {
    pub fn new(dsync: D, names: Vec<String>) -> DRWLock<L, D> {
        let (lockers, _) = dsync.get_lockers();
        DRWLock {
            names,
            dsync,
            write_locks: Arc::new(RwLock::new(vec!["".to_owned(); lockers.len()])),
            readers_locks: Default::default(),
            token: CancellationToken::new(),
            _phantom: Default::default(),
        }
    }

    pub async fn try_lock<F: FnOnce() + Send + 'static>(
        &mut self,
        lock_loss_callback: F,
        id: &str,
        source: &str,
        opts: Options,
    ) -> bool {
        self.lock_blocking(Some(lock_loss_callback), id, source, false, opts)
            .await
    }

    pub async fn rlock(&mut self, id: &str, source: &str) {
        self.lock_blocking(
            Some(|| {}),
            id,
            source,
            true,
            Options {
                timeout: DRW_MUTEX_INFINITE,
            },
        )
        .await;
    }

    pub async fn try_rlock<F: FnOnce() + Send + 'static>(
        &mut self,
        lock_loss_callback: F,
        id: &str,
        source: &str,
        opts: Options,
    ) -> bool {
        self.lock_blocking(Some(lock_loss_callback), id, source, true, opts)
            .await
    }

    pub async fn unlock(&mut self) {
        self.token.cancel();
    }

    pub async fn runlock(&mut self) {
        self.token.cancel();
    }

    async fn lock_blocking<F: FnOnce() + Send + 'static>(
        &mut self,
        lock_loss_callback: Option<F>,
        id: &str,
        source: &str,
        is_read_lock: bool,
        opts: Options,
    ) -> bool {
        let (lockers, _) = self.dsync.get_lockers();

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

        // Tolerance is not set, defaults to half of the locker clients.
        let mut tolerance = lockers.len() / 2;
        let mut quorum = lockers.len() - tolerance;
        if !is_read_lock && quorum == tolerance {
            // In situations for write locks, as a special case
            // to avoid split brains we make sure to acquire
            // quorum + 1 when tolerance is exactly half of the
            // total locker clients.
            quorum += 1;
            // So tolerance - 1.
            tolerance -= 1;
        }

        let (lockers, owner) = self.dsync.get_lockers();

        // Create lock array to capture the successful lockers
        let locks = Arc::new(RwLock::new(vec!["".to_owned(); lockers.len()]));

        let deadline = Instant::now() + opts.timeout;
        loop {
            let locked = {
                let lockers = lockers.clone();
                let owner = owner.clone();
                let mut locks = locks.clone();
                let names = self.names.clone();
                let readers_locks = self.readers_locks.clone();
                let write_locks = self.write_locks.clone();
                timeout_at(deadline, async move {
                    // Try to acquire the lock.
                    let locked = lock(
                        lockers,
                        &owner,
                        &mut locks,
                        id,
                        source,
                        is_read_lock,
                        tolerance,
                        quorum,
                        &names,
                        deadline,
                    )
                    .await;
                    if locked {
                        if is_read_lock {
                            readers_locks.write().await.push(locks.read().await.clone());
                        } else {
                            *write_locks.write().await = locks.read().await.clone();
                        }

                        trace!("lock_blocking {}/{} for {:?}: granted", &id, &source, names,);
                    }

                    locked
                })
                .await
            };
            match locked {
                Ok(locked) => {
                    if locked {
                        // Refresh lock continuously and cancel if there is no quorum in the lock anymore
                        let lockers = lockers.clone();
                        self.start_continuous_lock_refresh(
                            lock_loss_callback,
                            lockers,
                            owner,
                            id.to_owned(),
                            source.to_owned(),
                            quorum,
                        );
                        return locked;
                    }
                    let sleep_deadline =
                        Instant::now() + LOCK_RETRY_INTERVAL.mul_f64(rng.gen::<f64>());
                    if sleep_deadline < deadline {
                        tokio::time::sleep_until(sleep_deadline).await;
                    }
                }
                Err(_) => {
                    return false;
                }
            }
        }
    }

    async fn start_continuous_lock_refresh<F: FnOnce() + Send + 'static>(
        &mut self,
        lock_loss_callback: Option<F>,
        lockers: Vec<Arc<RwLock<L>>>,
        owner: String,
        id: String,
        source: String,
        quorum: usize,
    ) {
        let token = self.token.clone();
        let names = self.names.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(DRW_MUTEX_REFRESH_INTERVAL);
            loop {
                let lockers = lockers.clone();
                let owner = owner.clone();
                let id = id.clone();
                let source = source.clone();
                select! {
                    _ = token.cancelled() => {
                        return;
                    },
                    _ = interval.tick() => {
                        if let Ok(refreshed) = refresh(token.clone(), lockers, &owner, &id, &source, quorum, &names).await {
                            if !refreshed {
                                if let Some(lock_loss_callback) = lock_loss_callback {
                                    lock_loss_callback();
                                }
                                return;
                            }
                        }
                    },
                }
            }
        }); // do not await
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

struct RefreshResult {
    offline: bool,
    succeeded: bool,
}

async fn refresh<L: NetLocker + Send + Sync + 'static>(
    token: CancellationToken,
    lockers: Vec<Arc<RwLock<L>>>,
    owner: &str,
    id: &str,
    source: &str,
    quorum: usize,
    lock_names: &Vec<String>,
) -> anyhow::Result<bool> {
    // Create buffered channel of size equal to total number of nodes.
    let (tx, mut rx) = channel(lockers.len());

    // Send refresh request to all nodes.
    let mut handles = Vec::new();
    for locker in &lockers {
        let locker = locker.clone();
        let owner = owner.to_owned();
        let id = id.to_owned();
        let source = source.to_owned();
        let lock_names = lock_names.clone();
        let tx = tx.clone();
        let token = token.child_token();
        handles.push(tokio::spawn(async move {
            let args = LockArgs {
                uid: id,
                resources: lock_names,
                source,
                owner,
                quorum,
            };

            let mut locker = locker.write().await;
            let refreshed =
                timeout(DRW_MUTEX_REFRESH_CALL_TIMEOUT, locker.refresh(token, &args)).await;
            let err: anyhow::Error;
            match refreshed {
                Ok(refreshed) => match refreshed {
                    Ok(refreshed) => {
                        if refreshed {
                            let _ = tx
                                .send(RefreshResult {
                                    succeeded: true,
                                    offline: false,
                                })
                                .await;
                        } else {
                            let _ = tx
                                .send(RefreshResult {
                                    succeeded: false,
                                    offline: true,
                                })
                                .await;
                            trace!(
                                "dsync refresh returned false for {:?} at {}",
                                args,
                                locker.to_string()
                            );
                        }
                        return;
                    }
                    Err(e) => {
                        err = e;
                    }
                },
                Err(e) => {
                    err = e.into();
                }
            }
            let _ = tx
                .send(RefreshResult {
                    succeeded: false,
                    offline: false,
                })
                .await;
            trace!(
                "dsync unable to call refresh failed with {} for {:?} at {}",
                err,
                args,
                locker.to_string()
            );
        }));
    }

    // Wait until we have either
    // a) received all refresh responses
    // b) received too many refreshed for quorum to be still possible
    // c) timed out
    let (mut refresh_failed, mut refresh_succeeded) = (0, 0);
    for _ in 0..lockers.len() {
        select! {
            _ = token.cancelled() => {
                // Refreshing is canceled
                return Err(anyhow!("cancelled"));
            },
            r = rx.recv() => {
                match r {
                    Some(refresh) => {
                        if refresh.offline {
                            continue;
                        }
                        if refresh.succeeded {
                            refresh_succeeded += 1;
                        } else {
                            refresh_failed += 1;
                        }
                        if refresh_failed > quorum {
                            // We know that we are not going to succeed with refresh
                            break;
                        }
                    }
                    None => {
                        return Err(anyhow!("channel closed"));
                    }
                }
            },
        }
    }

    let mut refresh_quorum = refresh_succeeded >= quorum;
    if !refresh_quorum {
        refresh_quorum = refresh_failed < quorum;
    }

    // We may have some unused results in channel, release them async.
    tokio::spawn(async move {
        for r in futures::future::join_all(handles).await {
            r.unwrap(); // no task should panic
        }
        rx.close();
        while rx.recv().await.is_some() {}
    });

    Ok(refresh_quorum)
}

async fn lock<L: NetLocker + Send + Sync + 'static>(
    lockers: Vec<Arc<RwLock<L>>>,
    owner: &str,
    locks: &mut Arc<RwLock<Vec<String>>>,
    id: &str,
    source: &str,
    is_read_lock: bool,
    tolerance: usize,
    quorum: usize,
    lock_names: &Vec<String>,
    deadline: Instant,
) -> bool {
    let mut wlocks = locks.write().await;
    for l in wlocks.iter_mut() {
        l.clear();
    }

    let (tx, mut rx) = channel(lockers.len());

    let mut handles = Vec::new();
    for (index, locker) in lockers.iter().enumerate() {
        // Broadcast lock request to all nodes
        let id = id.to_owned();
        let source = source.to_owned();
        let owner = owner.to_owned();
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
            let _ = tx.send(g).await; // ignore error
        }));
    }

    // Wait until we have either
    // a) received all lock responses
    // b) received too many 'non-'locks for quorum to be still possible
    // c) timed out
    let mut locks_failed = 0;
    // Combined timeout for the lock attempt.
    let mut lock_deadline = Instant::now() + DRW_MUTEX_ACQUIRE_TIMEOUT;
    if lock_deadline > deadline {
        lock_deadline = deadline;
    }
    // Loop until we acquired all locks
    for i in 0..lockers.len() {
        let grant = timeout_at(lock_deadline, rx.recv()).await;
        match grant {
            Ok(grant) => {
                if let Some(grant) = grant {
                    if grant.is_locked() {
                        // Mark that this node has acquired the lock
                        wlocks[grant.index] = grant.lock_uid;
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

    let quorum_locked = check_quorum_locked(&*wlocks, quorum) && locks_failed <= tolerance;
    if !quorum_locked {
        trace!("Abandon since quorum was not met, so release all acquired locks now");
        drop(wlocks);
        if !release_all(tolerance, &owner, locks, is_read_lock, &lockers, lock_names).await {
            trace!("Unable to release acquired locks, stale locks might be present")
        }
    }

    // We may have some unused results in channel, release them async.
    let lock_names = lock_names.clone();
    let owner = owner.to_owned();
    tokio::spawn(async move {
        for r in futures::future::join_all(handles).await {
            r.unwrap(); // no task should panic
        }
        rx.close();
        while let Some(grant) = rx.recv().await {
            if grant.is_locked() {
                trace!("Releasing abandoned lock");
                send_release(
                    &mut *lockers[grant.index].write().await,
                    &owner,
                    &grant.lock_uid,
                    is_read_lock,
                    &lock_names,
                )
                .await;
            }
        }
    });

    true
}

async fn release_all<L: NetLocker + Send + Sync + 'static>(
    tolerance: usize,
    owner: &str,
    locks: &mut Arc<RwLock<Vec<String>>>,
    is_read_lock: bool,
    lockers: &Vec<Arc<RwLock<L>>>,
    names: &Vec<String>,
) -> bool {
    let mut handles = Vec::new();
    for (lock_id, locker) in lockers.iter().enumerate() {
        let locks = locks.clone();
        let locker = locker.clone();
        let owner = owner.to_owned();
        let names = names.clone();
        handles.push(tokio::spawn(async move {
            let mut locks = locks.write().await;
            let lock = &mut locks[lock_id];
            if is_locked(lock) {
                let mut locker = locker.write().await;
                if send_release(&mut *locker, &owner, lock, is_read_lock, &names).await {
                    lock.clear();
                }
            }
        }));
    }
    for r in futures::future::join_all(handles).await {
        r.unwrap(); // no task should panic
    }

    check_failed_unlocks(&locks.read().await, tolerance)
}

async fn send_release<L: NetLocker + Send + Sync + 'static>(
    locker: &mut L,
    owner: &str,
    uid: &str,
    is_read_lock: bool,
    names: &Vec<String>,
) -> bool {
    let args = LockArgs {
        owner: owner.to_owned(),
        uid: uid.to_owned(),
        resources: names.clone(),
        ..Default::default()
    };

    let r = timeout(
        DRW_MUTEX_UNLOCK_CALL_TIMEOUT,
        if is_read_lock {
            locker.runlock(&args)
        } else {
            locker.unlock(&args)
        },
    )
    .await;

    let unlock_type = if is_read_lock { "RUnlock" } else { "Unlock" };
    let err: anyhow::Error;
    match r {
        Ok(r) => match r {
            Ok(unlocked) => {
                return unlocked;
            }
            Err(e) => {
                err = e.into();
            }
        },
        Err(e) => {
            err = e.into();
        }
    }

    trace!(
        "dsync unable to call {} failed with '{}' for {:?} at {}\n",
        unlock_type,
        err,
        args,
        locker.to_string()
    );
    false
}

// Determines whether we have locked the required quorum of underlying locks or not.
fn check_quorum_locked(locks: &[String], quorum: usize) -> bool {
    locks.iter().filter(|&uid| is_locked(uid)).count() > quorum
}

// Determines whether we have sufficiently unlocked all
// resources to ensure no deadlocks for future callers
fn check_failed_unlocks(locks: &[String], tolerance: usize) -> bool {
    let unlocks_failed = locks.iter().filter(|&uid| is_locked(uid)).count();
    if locks.len() == tolerance * 2 {
        unlocks_failed >= tolerance
    } else {
        unlocks_failed > tolerance
    }
}
