use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use const_format::concatcp;
use lazy_static::lazy_static;
use strum::Display;

use crate::utils::minutes;
use crate::{globals, object};

mod stats;
mod utils;

pub use stats::*;
pub use utils::*;

const CACHE_BLK_SIZE: usize = 1 << 20;
const CACHE_GC_INTERVAL: Duration = minutes(30);
const WRITE_BACK_STATUS_HEADER: &str =
    concatcp!(globals::RESERVED_METADATA_PREFIX_LOWER, "write-back-status");
const WRITE_BACK_RETRY_HEADER: &str =
    concatcp!(globals::RESERVED_METADATA_PREFIX_LOWER, "write-back-retry");

#[derive(Display)]
enum CacheCommitStatus {
    // Cache writeback with backend is pending.
    #[strum(serialize = "pending")]
    Pending,
    // Cache writeback completed ok.
    #[strum(serialize = "complete")]
    Complete,
    // Cache writeback needs a retry.
    #[strum(serialize = "failed")]
    Failed,
}

pub struct CacheStorageInfo {
    pub total: u64,
    pub free: u64,
}

// Implements primitives for cache object API layer.
pub enum CacheObjectLayer {}

impl CacheObjectLayer {
    pub async fn get_object_and_info(
        &self,
        bucket: &str,
        object: &str,
        range: crate::http::HttpRange,
        header: &actix_web::http::HeaderMap,
        lock_type: object::LockType,
        opts: Option<object::ObjectOptions>,
    ) -> anyhow::Result<object::GetObjectReader> {
        todo!()
    }

    pub async fn get_object_info(
        &self,
        bucket: &str,
        object: &str,
        opts: Option<object::ObjectOptions>,
    ) -> anyhow::Result<object::ObjectInfo> {
        todo!()
    }

    pub async fn delete_object(
        &self,
        bucket: &str,
        object: &str,
        opts: Option<object::ObjectOptions>,
    ) -> anyhow::Result<object::ObjectInfo> {
        todo!()
    }

    pub async fn delete_objects(
        &self,
        bucket: &str,
        objects: &[object::ObjectToDelete],
        opts: Option<object::ObjectOptions>,
    ) -> anyhow::Result<Vec<object::DeletedObject>> {
        todo!()
    }

    pub async fn put_object(
        &self,
        bucket: &str,
        object: &str,
        data: &mut object::PutObjectReader,
        opts: Option<object::ObjectOptions>,
    ) -> anyhow::Result<object::ObjectInfo> {
        todo!()
    }

    pub async fn copy_object(
        &self,
        src_bucket: &str,
        src_object: &str,
        dst_bucket: &str,
        dst_object: &str,
        src_info: &object::ObjectInfo,
        src_opts: Option<object::ObjectOptions>,
        dst_opts: Option<object::ObjectOptions>,
    ) -> anyhow::Result<object::ObjectInfo> {
        todo!()
    }

    pub async fn storage_info(&self) -> anyhow::Result<crate::admin::StorageInfo> {
        todo!()
    }

    pub async fn cache_stats(&self) -> CacheStats {
        todo!()
    }
}

lazy_static! {
    static ref GLOBAL_CACHE_API: Arc<Mutex<Option<CacheObjectLayer>>> = Arc::new(Mutex::new(None));
}

pub fn get_cache_layer() -> MutexGuard<'static, Option<CacheObjectLayer>> {
    GLOBAL_CACHE_API.lock().unwrap()
}

pub fn set_cache_layer(api: CacheObjectLayer) {
    *GLOBAL_CACHE_API.lock().unwrap() = Some(api);
}
