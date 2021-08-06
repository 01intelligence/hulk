use std::time::Duration;

pub const KIB: usize = 1 << (1 * 10);
pub const MIB: usize = 1 << (2 * 10);

pub const MINUTE: u64 = 60;
pub const HOUR: u64 = MINUTE * 60;

pub const fn seconds(n: u64) -> Duration {
    Duration::from_secs(n)
}

pub const fn minutes(n: u64) -> Duration {
    Duration::from_secs(MINUTE * n)
}

pub const fn hours(n: u64) -> Duration {
    Duration::from_secs(HOUR * n)
}
