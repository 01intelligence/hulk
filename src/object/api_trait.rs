use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::*;

type CheckPreconditionFn = Box<dyn Fn(ObjectInfo) -> bool>;

// Object options for ObjectLayer object operations.
pub struct ObjectOptions {
    pub server_side_encryption: crate::encrypt::ServerSide,
    pub version_suspended: bool, // indicates if the bucket was previously versioned but is currently suspended.
    pub versioned: bool,         // indicates if the bucket is versioned
    pub walk_versions: bool,     // indicates if the we are interested in walking versions
    pub version_id: String,      // Specifies the versionID which needs to be overwritten or read
    pub mtime: DateTime<Utc>,    // Is only set in POST/PUT operations
    pub expires: DateTime<Utc>,  // Is only used in POST/PUT operations

    pub delete_marker: bool, // Is only set in DELETE operations for delete marker replication
    pub user_defined: HashMap<String, String>, // only set in case of POST/PUT operations
    pub part_number: isize,  // only useful in case of GetObject/HeadObject
    pub check_precondition_fn: CheckPreconditionFn, // only set during GetObject/HeadObject/CopyObjectPart precondition valuation
    pub delete_marker_replication_status: String,   // Is only set in DELETE operations
    pub version_purge_status: crate::storage::VersionPurgeStatus, // Is only set in DELETE operations for delete marker version to be permanently deleted.
    pub transition: TransitionOptions,

    pub no_lock: bool, // indicates to lower layers if the caller is expecting to hold locks.
    pub proxy_request: bool, // only set for GET/HEAD in active-active replication scenario
    pub proxy_header_set: bool, // only set for GET/HEAD in active-active replication scenario
    pub parent_is_object: Box<dyn Fn(&str, &str) -> bool>, // Used to verify if parent is an object.

    pub delete_prefix: bool, //  set true to enforce a prefix deletion, only application for DeleteObject API,

    // Use the maximum parity (N/2), used when saving server configuration files
    pub max_parity: bool,
}

pub struct TransitionOptions {
    pub status: String,
    pub tier: String,
    pub etag: String,
    pub restore_request: crate::bucket::RestoreRequest,
    pub restore_expiry: DateTime<Utc>,
    pub expire_restored: bool,
}

#[async_trait]
pub trait ObjectLayer {}
