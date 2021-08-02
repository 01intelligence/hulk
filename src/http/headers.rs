// Non standard S3 HTTP response constants
pub const X_CACHE: &str = "X-Cache";
pub const X_CACHE_LOOKUP: &str = "X-Cache-Lookup";

// Standard S3 HTTP request constants
pub const IF_MODIFIED_SINCE: &str = "If-Modified-Since";
pub const IF_UNMODIFIED_SINCE: &str = "If-Unmodified-Since";
pub const IF_MATCH: &str = "If-Match";
pub const IF_NONE_MATCH: &str = "If-None-Match";

// S3 storage class
pub const AMZ_STORAGE_CLASS: &str = "x-amz-storage-class";

// S3 object version ID
pub const AMZ_VERSION_ID: &str = "x-amz-version-id";
pub const AMZ_DELETE_MARKER: &str = "x-amz-delete-marker";

// S3 object tagging
pub const AMZ_OBJECT_TAGGING: &str = "X-Amz-Tagging";
pub const AMZ_TAG_COUNT: &str = "x-amz-tagging-count";
pub const AMZ_TAG_DIRECTIVE: &str = "X-Amz-Tagging-Directive";

// S3 transition restore
pub const AMZ_RESTORE: &str = "x-amz-restore";
pub const AMZ_RESTORE_EXPIRY_DAYS: &str = "X-Amz-Restore-Expiry-Days";
pub const AMZ_RESTORE_REQUEST_DATE: &str = "X-Amz-Restore-Request-Date";
pub const AMZ_RESTORE_OUTPUT_PATH: &str = "x-amz-restore-output-path";

// S3 extensions
pub const AMZ_COPY_SOURCE_IF_MODIFIED_SINCE: &str = "x-amz-copy-source-if-modified-since";
pub const AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE: &str = "x-amz-copy-source-if-unmodified-since";

pub const AMZ_COPY_SOURCE_IF_NONE_MATCH: &str = "x-amz-copy-source-if-none-match";
pub const AMZ_COPY_SOURCE_IF_MATCH: &str = "x-amz-copy-source-if-match";

pub const AMZ_COPY_SOURCE: &str = "X-Amz-Copy-Source";
pub const AMZ_COPY_SOURCE_VERSION_ID: &str = "X-Amz-Copy-Source-Version-Id";
pub const AMZ_COPY_SOURCE_RANGE: &str = "X-Amz-Copy-Source-Range";
pub const AMZ_METADATA_DIRECTIVE: &str = "X-Amz-Metadata-Directive";
pub const AMZ_OBJECT_LOCK_MODE: &str = "X-Amz-Object-Lock-Mode";
pub const AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE: &str = "X-Amz-Object-Lock-Retain-Until-Date";
pub const AMZ_OBJECT_LOCK_LEGAL_HOLD: &str = "X-Amz-Object-Lock-Legal-Hold";
pub const AMZ_OBJECT_LOCK_BYPASS_GOVERNANCE: &str = "X-Amz-Bypass-Governance-Retention";
pub const AMZ_BUCKET_REPLICATION_STATUS: &str = "X-Amz-Replication-Status";
pub const AMZ_SNOWBALL_EXTRACT: &str = "X-Amz-Meta-Snowball-Auto-Extract";

// Multipart parts count
pub const AMZ_MP_PARTS_COUNT: &str = "x-amz-mp-parts-count";

// Object date/time of expiration
pub const AMZ_EXPIRATION: &str = "x-amz-expiration";

// Dummy putBucketACL
pub const AMZ_ACL: &str = "x-amz-acl";

// Signature V4 related contants.
pub const AMZ_CONTENT_SHA256: &str = "X-Amz-Content-Sha256";
pub const AMZ_DATE: &str = "X-Amz-Date";
pub const AMZ_ALGORITHM: &str = "X-Amz-Algorithm";
pub const AMZ_EXPIRES: &str = "X-Amz-Expires";
pub const AMZ_SIGNED_HEADERS: &str = "X-Amz-SignedHeaders";
pub const AMZ_SIGNATURE: &str = "X-Amz-Signature";
pub const AMZ_CREDENTIAL: &str = "X-Amz-Credential";
pub const AMZ_SECURITY_TOKEN: &str = "X-Amz-Security-Token";
pub const AMZ_DECODED_CONTENT_LENGTH: &str = "X-Amz-Decoded-Content-Length";

pub const AMZ_META_UNENCRYPTED_CONTENT_LENGTH: &str = "X-Amz-Meta-X-Amz-Unencrypted-Content-Length";
pub const AMZ_META_UNENCRYPTED_CONTENT_MD5: &str = "X-Amz-Meta-X-Amz-Unencrypted-Content-Md5";

// AWS server-side encryption headers for SSE-S3, SSE-KMS and SSE-C.
pub const AMZ_SERVER_SIDE_ENCRYPTION: &str = "X-Amz-Server-Side-Encryption";
pub const AMZ_SERVER_SIDE_ENCRYPTION_KMS_ID: &str = "-Aws-Kms-Key-Id";
pub const AMZ_SERVER_SIDE_ENCRYPTION_KMS_CONTEXT: &str = "-Context";
pub const AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: &str = "-Customer-Algorithm";
pub const AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY: &str = "-Customer-Key";
pub const AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5: &str = "-Customer-Key-Md5";
pub const AMZ_SERVER_SIDE_ENCRYPTION_COPY_CUSTOMER_ALGORITHM: &str =
    "X-Amz-Copy-Source-Server-Side-Encryption-Customer-Algorithm";
pub const AMZ_SERVER_SIDE_ENCRYPTION_COPY_CUSTOMER_KEY: &str =
    "X-Amz-Copy-Source-Server-Side-Encryption-Customer-Key";
pub const AMZ_SERVER_SIDE_ENCRYPTION_COPY_CUSTOMER_KEY_MD5: &str =
    "X-Amz-Copy-Source-Server-Side-Encryption-Customer-Key-Md5";

pub const AMZ_ENCRYPTION_AES: &str = "AES256";
pub const AMZ_ENCRYPTION_KMS: &str = "aws:kms";

// Signature v2 related constants
pub const AMZ_SIGNATURE_V2: &str = "Signature";
pub const AMZ_ACCESS_KEY_ID: &str = "AWSAccessKeyId";

// Response request id.
pub const AMZ_REQUEST_ID: &str = "x-amz-request-id";

// Deployment id.
pub const HULK_DEPLOYMENT_ID: &str = "x-hulk-deployment-id";

// Server-Status
pub const HULK_SERVER_STATUS: &str = "x-hulk-server-status";

// Delete special flag to force delete a bucket or a prefix
pub const HULK_FORCE_DELETE: &str = "x-hulk-force-delete";

// Header indicates if the mtime should be preserved by client
pub const HULK_SOURCE_MTIME: &str = "x-hulk-source-mtime";

// Header indicates if the etag should be preserved by client
pub const HULK_SOURCE_ETAG: &str = "x-hulk-source-etag";

// Writes expected write quorum
pub const HULK_WRITE_QUORUM: &str = "x-hulk-write-quorum";

// Reports number of drives currently healing
pub const HULK_HEALING_DRIVES: &str = "x-hulk-healing-drives";

// Header indicates if the delete marker should be preserved by client
pub const HULK_SOURCE_DELETE_MARKER: &str = "x-hulk-source-deletemarker";

// Header indicates if the delete marker version needs to be purged.
pub const HULK_SOURCE_DELETE_MARKER_DELETE: &str = "x-hulk-source-deletemarker-delete";

// Header indicates permanent delete replication status.
pub const HULK_DELETE_REPLICATION_STATUS: &str = "X-Hulk-Replication-Delete-Status";
// Header indicates delete-marker replication status.
pub const HULK_DELETE_MARKER_REPLICATION_STATUS: &str = "X-Hulk-Replication-DeleteMarker-Status";
// Header indicates if its a GET/HEAD proxy request for active-active replication
pub const HULK_SOURCE_PROXY_REQUEST: &str = "X-Hulk-Source-Proxy-Request";
// Header indicates that this request is a replication request to create a REPLICA
pub const HULK_SOURCE_REPLICATION_REQUEST: &str = "X-Hulk-Source-Replication-Request";
// Header indicates replication reset status.
pub const HULK_REPLICATION_RESET_STATUS: &str = "X-Hulk-Replication-Reset-Status";

// predicted date/time of transition
pub const HULK_TRANSITION: &str = "X-Hulk-Transition";

// Common http query params S3 API
pub const VERSION_ID: &str = "versionId";

pub const PART_NUMBER: &str = "partNumber";

pub const UPLOAD_ID: &str = "uploadId";
