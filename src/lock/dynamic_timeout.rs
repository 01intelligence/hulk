use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use lazy_static::lazy_static;
use tokio::sync::Mutex;
use tokio::time::Duration;

const DYNAMIC_TIMEOUT_INCREASE_THRESHOLD_PCT: f64 = 0.33; // Upper threshold for failures in order to increase timeout
const DYNAMIC_TIMEOUT_DECREASE_THRESHOLD_PCT: f64 = 0.10; // Lower threshold for failures in order to decrease timeout
const DYNAMIC_TIMEOUT_LOG_SIZE: usize = 16;

lazy_static! {
    static ref MAX_DURATION: Duration = Duration::MAX;
    static ref MAX_DYNAMIC_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60); // Never set timeout bigger than this.
}

pub struct DynamicTimeout {
    timeout: Arc<AtomicU64>,
    minimum: u64,
    entries: Arc<AtomicUsize>,
    logs: Mutex<[Duration; DYNAMIC_TIMEOUT_LOG_SIZE]>,
}

impl DynamicTimeout {
    pub fn new(timeout: Duration, mut minimum: Duration) -> DynamicTimeout {
        if timeout <= Duration::ZERO || minimum <= Duration::ZERO {
            panic!("negative or zero timeout");
        }
        if minimum > timeout {
            minimum = timeout;
        }
        DynamicTimeout {
            timeout: Arc::new(AtomicU64::new(timeout.as_nanos() as u64)),
            minimum: minimum.as_nanos() as u64,
            entries: Default::default(),
            logs: Mutex::new([Duration::ZERO; DYNAMIC_TIMEOUT_LOG_SIZE]),
        }
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_nanos(self.timeout.load(Ordering::Relaxed))
    }

    pub async fn log_success(&mut self, duration: Duration) {
        self.log_entry(duration).await
    }

    pub async fn log_failure(&mut self) {
        self.log_entry(*MAX_DURATION).await
    }

    async fn log_entry(&mut self, duration: Duration) {
        if duration < Duration::ZERO {
            return;
        }
        let entries = self.entries.fetch_add(1, Ordering::SeqCst) + 1;
        let index = entries - 1;
        if index < DYNAMIC_TIMEOUT_LOG_SIZE {
            let mut logs = self.logs.lock().await;
            logs[index] = duration;

            if entries != DYNAMIC_TIMEOUT_LOG_SIZE {
                return;
            }

            self.entries.store(0, Ordering::Relaxed);

            let logs_copy = (*logs).clone();
            drop(logs);
            self.adjust(logs_copy);
        }
    }

    fn adjust(&mut self, logs: [Duration; DYNAMIC_TIMEOUT_LOG_SIZE]) {
        let mut failures = 0;
        let mut max = Duration::ZERO;
        for &dur in &logs {
            if dur == *MAX_DURATION {
                failures += 1;
            } else if dur > max {
                max = dur;
            }
        }

        let fail_percent = (failures as f64) / (logs.len() as f64);
        if fail_percent > DYNAMIC_TIMEOUT_INCREASE_THRESHOLD_PCT {
            // We are hitting the timeout too often, so increase the timeout by 25%
            let mut timeout = self.timeout.load(Ordering::Relaxed) * 125 / 100;
            timeout = timeout.max(MAX_DYNAMIC_TIMEOUT.as_nanos() as u64);
            timeout = timeout.min(self.minimum);
            self.timeout.store(timeout, Ordering::Relaxed);
        } else if fail_percent < DYNAMIC_TIMEOUT_DECREASE_THRESHOLD_PCT {
            // We are hitting the timeout relatively few times,
            // so decrease the timeout towards 25 % of maximum time spent.
            let max = (max * 125 / 100).as_nanos() as u64;
            let mut timeout = self.timeout.load(Ordering::Relaxed);
            if max < timeout {
                // Move 50% toward the max.
                timeout = (max + timeout) / 2;
            }
            timeout = timeout.min(self.minimum);
            self.timeout.store(timeout, Ordering::Relaxed);
        }
    }
}
