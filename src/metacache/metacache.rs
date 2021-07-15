use std::time::Duration;

pub(super) enum ScanStatus {
    None,
    Started,
    Success,
    Error,
}

// Time in which the initiator of a scan must have reported back.
pub const METACACHE_MAX_RUNNING_AGE: Duration = crate::utils::minutes(1);

// The number of file/directory entries to have in each block.
pub const METACACHE_BLOCK_SIZE: usize = 5000;

// Controls whether prefixes on dirty paths are always shared.
// This will make `test/a` and `test/b` share listings if they are concurrent.
// Enabling this will make cache sharing more likely and cause less IO,
// but may cause additional latency to some calls.
pub const METACACHE_SHARE_PREFIX: bool = false;
