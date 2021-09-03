use std::collections::HashMap;

use super::*;

/// Keeps track of all caches generated for a bucket.
pub struct BucketMetaCache {
    bucket: String,
    caches: HashMap<String, MetaCache>,
    caches_root: HashMap<String, Vec<String>>,
    updated: bool,
    transient: bool,
}
