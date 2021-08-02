use std::fmt;
use std::path::Path;

use thiserror::Error;

use crate::errors::AsError;

#[derive(Debug, Error)]
pub struct GenericError {
    pub bucket: String,
    pub object: String,
    pub version_id: String,
    #[source]
    pub err: Option<anyhow::Error>,
}

impl GenericError {
    fn format_err(&self) -> String {
        match &self.err {
            Some(err) => format!(": ({})", err),
            None => "".to_string(),
        }
    }
}

impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err = match &self.err {
            Some(err) => format!(": ({})", err),
            None => "".to_string(),
        };
        write!(
            f,
            "{}/{}({}): {}",
            self.bucket, self.object, self.version_id, err,
        )
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApiError {
    #[error("The request signature we calculated does not match the signature you provided. Check your key and signing method.")]
    SignatureDoesNotMatch,
    #[error("Storage reached its minimum free disk threshold.")]
    StorageFull,
    #[error("Please reduce your request rate")]
    SlowDown,
    #[error("Storage resources are insufficient for the read operation {}/{}", .0.bucket, .0.object)]
    InsufficientReadQuorum(GenericError),
    #[error("Storage resources are insufficient for the write operation {}/{}", .0.bucket, .0.object)]
    InsufficientWriteQuorum(GenericError),
    #[error("Invalid arguments provided for {}/{}{}", .0.bucket, .0.object, .0.format_err())]
    InvalidArgument(GenericError),
    #[error("Bucket not found: {}", .0.bucket)]
    BucketNotFound(GenericError),
    #[error("The requested bucket name is not available. The bucket namespace is shared by all users of the system. Please select a different name and try again.")]
    BucketAlreadyExists(GenericError),
    #[error("Bucket already owned by you: {}", .0.bucket)]
    BucketAlreadyOwnedByYou(GenericError),
    #[error("Bucket not empty: {}", .0.bucket)]
    BucketNotEmpty(GenericError),
    #[error("Invalid version id: {}/{}({})", .0.bucket, .0.object, .0.version_id)]
    InvalidVersionID(GenericError),
    #[error("Version not found: {}/{}({})", .0.bucket, .0.object, .0.version_id)]
    VersionNotFound(GenericError),
    #[error("Object not found: {}/{}", .0.bucket, .0.object)]
    ObjectNotFound(GenericError),
    #[error("Method not allowed: {}/{}", .0.bucket, .0.object)]
    MethodNotAllowed(GenericError),
    #[error("Object {}/{} already exists", .0.bucket, .0.object)]
    ObjectAlreadyExists(GenericError),
    #[error("Object exists on {} as directory {}", .0.bucket, .0.object)]
    ObjectExistsAsDirectory(GenericError),
    #[error("Prefix access is denied: {}/{}", .0.bucket, .0.object)]
    PrefixAccessDenied(GenericError),
    #[error("Parent is object: {}/{}", .0.bucket, Path::new(&.0.object).parent().map(|v| v.to_str()).flatten().unwrap_or(""))]
    ParentIsObject(GenericError),
    #[error("Bucket exists: {}", .0.bucket)]
    BucketExists(GenericError),
    #[error("Invalid combination of uploadID marker '{}' and marker '{}'", .upload_id_marker, .key_marker)]
    InvalidUploadIDKeyCombination {
        upload_id_marker: String,
        key_marker: String,
    },
    #[error("Invalid combination of marker '{}' and prefix '{}'", .marker, .prefix)]
    InvalidMarkerPrefixCombination { marker: String, prefix: String },
    #[error("No bucket policy configuration found for bucket: {}", .0.bucket)]
    BucketPolicyNotFound(GenericError),
    #[error("No bucket lifecycle configuration found for bucket : {}", .0.bucket)]
    BucketLifecycleNotFound(GenericError),
    #[error("No bucket encryption configuration found for bucket: {}", .0.bucket)]
    BucketSSEConfigNotFound(GenericError),
    #[error("No bucket tags found for bucket: {}", .0.bucket)]
    BucketTaggingNotFound(GenericError),
    #[error("No bucket object lock configuration found for bucket: {}", .0.bucket)]
    BucketObjectLockConfigNotFound(GenericError),
    #[error("No quota config found for bucket : {}", .0.bucket)]
    BucketQuotaConfigNotFound(GenericError),
    #[error("Bucket quota exceeded for bucket: {}", .0.bucket)]
    BucketQuotaExceeded(GenericError),
    #[error("The replication configuration was not found: {}", .0.bucket)]
    BucketReplicationConfigNotFound(GenericError),
    #[error("Destination bucket does not exist: {}", .0.bucket)]
    BucketRemoteDestinationNotFound(GenericError),
    #[error("Destination bucket does not have object lock enabled: {}", .0.bucket)]
    BucketReplicationDestinationMissingLock(GenericError),
    #[error("Remote target not found: {}", .0.bucket)]
    BucketRemoteTargetNotFound(GenericError),
    #[error("Remote service endpoint or target bucket {} not available: {}", .0.bucket, .0.format_err())]
    BucketRemoteConnectionErr(GenericError),
    #[error("Remote already exists for this bucket: {}", .0.bucket)]
    BucketRemoteAlreadyExists(GenericError),
    #[error("Remote with this label already exists for this bucket: {}", .0.bucket)]
    BucketRemoteLabelInUse(GenericError),
    #[error("Remote ARN type not valid: {}", .0.bucket)]
    BucketRemoteArnTypeInvalid(GenericError),
    #[error("Remote ARN has invalid format: {}", .0.bucket)]
    BucketRemoteArnInvalid(GenericError),
    #[error("Replication configuration exists with this ARN: {}", .0.bucket)]
    BucketRemoteRemoveDisallowed(GenericError),
    #[error("Remote target does not have versioning enabled: {}", .0.bucket)]
    BucketRemoteTargetNotVersioned(GenericError),
    #[error("Replication source does not have versioning enabled: {}", .0.bucket)]
    BucketReplicationSourceNotVersioned(GenericError),
    #[error("Transition storage class not found")]
    TransitionStorageClassNotFound(GenericError),
    #[error("The operation is not valid for the current state of the object {0}")]
    InvalidObjectState(GenericError),
    #[error("Bucket name invalid: {}", .0.bucket)]
    BucketNameInvalid(GenericError),
    #[error("Object name invalid: {}/{}", .0.bucket, .0.object)]
    ObjectNameInvalid(GenericError),
    #[error("Object name too long: {}/{}", .0.bucket, .0.object)]
    ObjectNameTooLong(GenericError),
    #[error("Object name contains forward slash as pefix: {}/{}", .0.bucket, .0.object)]
    ObjectNamePrefixAsSlash(GenericError),
    #[error("All access to this object has been disabled")]
    AllAccessDisabled(GenericError),
    #[error("{}/{} has incomplete body", .0.bucket, .0.object)]
    IncompleteBody(GenericError),
    #[error("The requested range \"bytes {offset_begin} -> {offset_end} of {resource_size}\" is not satisfiable.")]
    InvalidRange {
        offset_begin: usize,
        offset_end: usize,
        resource_size: usize,
    },
    #[error("Size of the object greater than what is allowed(5G)")]
    ObjectTooLarge(GenericError),
    #[error("Size of the object less than what is expected")]
    ObjectTooSmall(GenericError),
    #[error("Operation timed out")]
    OperationTimedOut,
    #[error("Malformed upload id {upload_id}")]
    MalformedUploadID { upload_id: String },
    #[error("Invalid upload id {upload_id}")]
    InvalidUploadID {
        bucket: String,
        object: String,
        upload_id: String,
    },
    #[error("Specified part could not be found. PART_NUMBER {part_number}, Expected {exp_etag}, got {got_etag}")]
    InvalidPart {
        part_number: String,
        exp_etag: String,
        got_etag: String,
    },
    #[error("Part size for {part_number} should be at least 5MB")]
    PartTooSmall {
        part_size: usize,
        part_number: usize,
        part_etag: String,
    },
    #[error("Part size bigger than the allowed limit")]
    PartTooBig,
    #[error("ETag of the object has changed")]
    InvalidEtag,
    #[error("{message}")]
    NotImplemented { message: String },
    #[error("Unsupported headers in Metadata")]
    UnsupportedMetadata,
    #[error("Backend down")]
    BackendDown,
    #[error("At least one of the pre-conditions you specified did not hold")]
    PreConditionFailed,
}

pub fn is_bucket_not_found(err: &anyhow::Error) -> bool {
    match err.as_error::<ApiError>() {
        Some(e) => {
            if let ApiError::BucketNotFound(_) = e {
                return true;
            }
        }
        _ => {}
    }
    false
}

pub fn is_object_not_found(err: &anyhow::Error) -> bool {
    match err.as_error::<ApiError>() {
        Some(e) => {
            if let ApiError::ObjectNotFound(_) = e {
                return true;
            }
        }
        _ => {}
    }
    false
}

pub fn is_version_not_found(err: &anyhow::Error) -> bool {
    match err.as_error::<ApiError>() {
        Some(e) => {
            if let ApiError::VersionNotFound(_) = e {
                return true;
            }
        }
        _ => {}
    }
    false
}

pub fn is_signature_not_match(err: &anyhow::Error) -> bool {
    match err.as_error::<ApiError>() {
        Some(e) => {
            if let ApiError::SignatureDoesNotMatch = e {
                return true;
            }
        }
        _ => {}
    }
    false
}
