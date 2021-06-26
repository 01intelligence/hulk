use tokio::time::{timeout_at, Duration, Instant};

use crate::utils::{rng_seed_now, sleep_until};

const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Default)]
pub struct TimedRWLock {
    is_write_lock: bool,
    refs: u32,
}

impl TimedRWLock {
    pub async fn lock(&mut self, timeout: Duration) -> bool {
        self.lock_loop(timeout, true).await
    }

    pub async fn rlock(&mut self, timeout: Duration) -> bool {
        self.lock_loop(timeout, false).await
    }

    pub fn unlock(&mut self) {
        if !self.unlock_internal(true) {
            panic!("trying to unlock while no lock is active");
        }
    }

    pub fn runlock(&mut self) {
        if !self.unlock_internal(false) {
            panic!("trying to runlock while no rlock is active");
        }
    }

    async fn lock_loop(&mut self, timeout: Duration, is_write_lock: bool) -> bool {
        let rng = &mut rng_seed_now();
        let deadline = Instant::now() + timeout;
        loop {
            let r = timeout_at(deadline, async {
                if self.lock_internal(is_write_lock) {
                    return true;
                }
                sleep_until(deadline, LOCK_RETRY_INTERVAL, Some(rng)).await;
                return false;
            })
            .await;
            match r {
                Ok(locked) => {
                    if locked {
                        return true;
                    }
                }
                Err(_) => {
                    return false; // timeout
                }
            }
        }
    }

    fn lock_internal(&mut self, is_write_lock: bool) -> bool {
        let mut locked = false;
        if is_write_lock {
            if self.refs == 0 && !self.is_write_lock {
                self.refs = 1;
                self.is_write_lock = true;
                locked = true;
            }
        } else {
            if !self.is_write_lock {
                self.refs += 1;
                locked = true;
            }
        }
        locked
    }

    fn unlock_internal(&mut self, is_write_lock: bool) -> bool {
        let mut unlocked = false;
        if is_write_lock {
            if self.is_write_lock && self.refs == 1 {
                self.refs = 0;
                self.is_write_lock = false;
                unlocked = true;
            }
        } else {
            if !self.is_write_lock && self.refs > 1 {
                self.refs -= 1;
                unlocked = true;
            }
        }
        unlocked
    }
}
