use std::collections::HashMap;

use chrono::{DateTime, Utc};

pub enum BackendType {
    Unknown,
    Fs,
    Erasure,
    Gateway,
}

pub struct ObjectInfo {
    // Name of the bucket.
    pub bucket: String,
    // Name of the object.
    pub name: String,
    // Date and time when the object was last modified.
    pub mod_type: DateTime<Utc>,
    // Total object size.
    pub size: i64,
    // IsDir indicates if the object is prefix.
    pub is_dir: bool,
    // Hex encoded unique entity tag of the object.
    pub etag: String,
    // The ETag stored in the gateway backend
    pub inner_etag: String,
    // Version ID of this object.
    pub version_id: String,
    // Indicates if this is the latest current version
    // latest can be true for delete marker or a version.
    pub is_latest: bool,
    // Indicates if the versionId corresponds
    // to a delete marker on an object.
    pub delete_marker: bool,

    // Indicates if transition is complete/pending
    pub transition_status: String,
    // Name of transitioned object on remote tier
    transitioned_obj_name: String,
    // VersionID on the the remote tier
    transition_version_id: String,
    // Name of remote tier object has transitioned to
    pub transition_tier: String,

    // Indicates date a restored object expires
    pub restore_expires: DateTime<Utc>,

    // Indicates if a restore is in progress
    pub restore_ongoing: bool,

    // A standard MIME type describing the format of the object.
    pub content_type: String,

    // Specifies what content encodings have been applied to the object and thus
    // what decoding mechanisms must be applied to obtain the object referenced
    // by the Content-Type header field.
    pub content_encoding: String,

    // Date and time at which the object is no longer able to be cached
    pub expires: DateTime<Utc>,

    // Sets status of whether this is a cache hit/miss
    pub cache_status: crate::diskcache::CacheStatus,
    // Sets whether a cacheable response is present in the cache
    pub cache_lookup_status: crate::diskcache::CacheStatus,

    // Specify object storage class
    pub storage_class: String,

    pub replication_status: crate::bucket::replication::Status,
    // User-Defined metadata
    pub user_defined: HashMap<String, String>,

    // User-Defined object tags
    pub user_tags: String,

    // List of individual parts, maximum size of upto 10,000
    pub parts: Vec<crate::xl_storage::ObjectPartInfo>,

    // Implements writer and reader used by CopyObject API
    // pub Writer:        io.WriteCloser,
    // pub Reader:       *hash.Reader,
    // pub PutObjReader :*PutObjReader,
    metadata_only: bool,
    version_only: bool, // adds a new version, only used by CopyObject
    key_rotation: bool,

    // Date and time when the object was last accessed.
    pub acc_time: DateTime<Utc>,

    // Indicates object on disk is in legacy data format
    pub legacy: bool,

    // Indicates which backend filled this structure
    backend_type: BackendType,

    pub version_purge_status: crate::storage::VersionPurgeStatus,

    // The total count of all versions of this object
    pub num_versions: isize,
    //  The modtime of the successor object version if any
    pub successor_mod_time: DateTime<Utc>,
}
