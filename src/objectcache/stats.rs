use std::sync::atomic::{AtomicU64, Ordering};

// Represents cache disk statistics
// such as current disk usage and available.
pub struct CacheDiskStats {
    pub usage_size: u64,     // used cache size
    pub total_capacity: u64, // total cache disk capacity
    pub usage_state: i32, // indicates if usage is high or low, if high value is '1', if low its '0'
    pub usage_percent: u64, // indicates the current usage percentage of this cache disk
    pub dir: String,
}

impl CacheDiskStats {
    pub fn get_usage_state_string(&self) -> &'static str {
        if self.usage_state == 0 {
            "low"
        } else {
            "high"
        }
    }
}

// Represents bytes served from cache,
// cache hits and cache misses
pub struct CacheStats {
    bytes_served: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    get_disk_stats: Box<dyn Fn() -> CacheDiskStats>,
}

impl CacheStats {
    pub fn inc_bytes_served(&mut self) {
        let _ = self.bytes_served.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_hits(&mut self) {
        let _ = self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_misses(&mut self) {
        let _ = self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn bytes_served(&self) -> u64 {
        self.bytes_served.load(Ordering::Relaxed)
    }

    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }
}
