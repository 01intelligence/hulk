use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use tokio::io::AsyncRead;

use super::*;

type CheckPreconditionFn = Box<dyn Fn(ObjectInfo) -> bool>;

// Object options for ObjectLayer object operations.
#[derive(Default)]
pub struct ObjectOptions {
    pub server_side_encryption: Option<crate::encrypt::ServerSide>,
    pub version_suspended: bool, // indicates if the bucket was previously versioned but is currently suspended.
    pub versioned: bool,         // indicates if the bucket is versioned
    pub walk_versions: bool,     // indicates if the we are interested in walking versions
    pub version_id: String,      // Specifies the versionID which needs to be overwritten or read
    pub mtime: Option<DateTime<Utc>>, // Is only set in POST/PUT operations
    pub expires: Option<DateTime<Utc>>, // Is only used in POST/PUT operations

    pub delete_marker: bool, // Is only set in DELETE operations for delete marker replication
    pub user_defined: HashMap<String, String>, // only set in case of POST/PUT operations
    pub part_number: isize,  // only useful in case of GetObject/HeadObject
    pub check_precondition_fn: Option<CheckPreconditionFn>, // only set during GetObject/HeadObject/CopyObjectPart precondition valuation
    pub delete_marker_replication_status: String,           // Is only set in DELETE operations
    pub version_purge_status: Option<crate::storage::VersionPurgeStatus>, // Is only set in DELETE operations for delete marker version to be permanently deleted.
    pub transition: TransitionOptions,

    pub no_lock: bool, // indicates to lower layers if the caller is expecting to hold locks.
    pub proxy_request: bool, // only set for GET/HEAD in active-active replication scenario
    pub proxy_header_set: bool, // only set for GET/HEAD in active-active replication scenario
    pub parent_is_object: Option<Box<dyn Fn(&str, &str) -> bool>>, // Used to verify if parent is an object.

    pub delete_prefix: bool, //  set true to enforce a prefix deletion, only application for DeleteObject API,

    // Use the maximum parity (N/2), used when saving server configuration files
    pub max_parity: bool,
}

#[derive(Default)]
pub struct TransitionOptions {
    pub status: String,
    pub tier: String,
    pub etag: String,
    pub restore_request: crate::bucket::RestoreRequest,
    pub restore_expiry: Option<DateTime<Utc>>,
    pub expire_restored: bool,
}

// Represents required locking for ObjectLayer operations.
pub enum LockType {
    None,
    Read,
    Write,
}

// Implements primitives for object API layer.
pub enum ObjectLayer {}

impl ObjectLayer {
    // Locking operations on object.

    pub async fn new_ns_lock<'a>(
        &'a mut self,
        bucket: &str,
        objects: &[&str],
    ) -> Box<dyn crate::lock::RWLocker + 'a> {
        todo!()
    }

    // Storage operations.

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    pub async fn ns_scanner(&self) -> anyhow::Result<()> {
        todo!()
    }

    pub async fn backend_info(&self) -> crate::admin::BackendInfo {
        todo!()
    }

    // Bucket operations.

    // Object operations.

    pub async fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
        marker: &str,
        delimiter: &str,
        max_keys: usize,
    ) -> anyhow::Result<ListObjectsInfo> {
        todo!()
    }

    pub async fn list_objects_v2(
        &self,
        bucket: &str,
        prefix: &str,
        continuation_token: &str,
        delimiter: &str,
        max_keys: usize,
        fetch_owner: bool,
        start_after: bool,
    ) -> anyhow::Result<ListObjectsV2Info> {
        todo!()
    }

    pub async fn get_object_and_info(
        &self,
        bucket: &str,
        object: &str,
        range: crate::http::HttpRange,
        header: &actix_web::http::HeaderMap,
        lock_type: LockType,
        opts: Option<ObjectOptions>,
    ) -> anyhow::Result<GetObjectReader> {
        todo!()
    }

    pub async fn get_object_info(
        &self,
        bucket: &str,
        object: &str,
        opts: Option<ObjectOptions>,
    ) -> anyhow::Result<ObjectInfo> {
        todo!()
    }

    pub async fn put_object(
        &self,
        bucket: &str,
        object: &str,
        data: &mut PutObjectReader,
        opts: Option<ObjectOptions>,
    ) -> anyhow::Result<ObjectInfo> {
        todo!()
    }

    pub async fn delete_object(
        &self,
        bucket: &str,
        object: &str,
        opts: Option<ObjectOptions>,
    ) -> anyhow::Result<ObjectInfo> {
        todo!()
    }

    // Multipart operations.

    // Policy operations.

    // Supported operations check.

    // Healing operations.

    // Backend related metrics.

    // Metadata operations.

    // ObjectTagging operations.
}

lazy_static! {
    static ref GLOBAL_OBJECT_API: Arc<Mutex<Option<ObjectLayer>>> = Arc::new(Mutex::new(None));
}

pub fn get_object_layer() -> MutexGuard<'static, Option<ObjectLayer>> {
    GLOBAL_OBJECT_API.lock().unwrap()
}

pub fn set_object_layer(api: ObjectLayer) {
    *GLOBAL_OBJECT_API.lock().unwrap() = Some(api);
}
