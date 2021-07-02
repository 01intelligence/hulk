use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TypedError {
    // errInvalidArgument means that input argument is invalid.
    #[error("Invalid arguments specified")]
    InvalidArgument,

    // errMethodNotAllowed means that method is not allowed.
    #[error("Method not allowed")]
    MethodNotAllowed,

    // errSignatureMismatch means signature did not match.
    #[error("Signature does not match")]
    SignatureMismatch,

    // used when we deal with data larger than expected
    #[error("Data size larger than expected")]
    SizeUnexpected,

    // used when we deal with data with unknown size
    #[error("Data size is unspecified")]
    SizeUnspecified,

    // When upload object size is greater than 5G in a single PUT/POST operation.
    #[error("Object size larger than allowed limit")]
    DataTooLarge,

    // When upload object size is less than what was expected.
    #[error("Object size smaller than expected")]
    DataTooSmall,

    // errServerNotInitialized - server not initialized.
    #[error("Server not initialized, please try again")]
    ServerNotInitialized,

    // errRPCAPIVersionUnsupported - unsupported rpc API version.
    #[error("Unsupported rpc API version")]
    RPCAPIVersionUnsupported,

    // errServerTimeMismatch - server times are too far apart.
    #[error("Server times are too far apart")]
    ServerTimeMismatch,

    // errInvalidBucketName - bucket name is reserved for MinIO, usually
    // returned for 'minio', '.minio.sys', buckets with capital letters.
    #[error("The specified bucket is not valid")]
    InvalidBucketName,

    // errInvalidRange - returned when given range value is not valid.
    #[error("Invalid range")]
    InvalidRange,

    // errInvalidRangeSource - returned when given range value exceeds
    // the source object size.
    #[error("Range specified exceeds source object size")]
    InvalidRangeSource,

    // error returned by disks which are to be initialized are waiting for the
    // first server to initialize them in distributed set to initialize them.
    #[error("Not first disk")]
    NotFirstDisk,

    // error returned by first disk waiting to initialize other servers.
    #[error("Waiting on other disks")]
    FirstDiskWait,

    // error returned when a bucket already exists
    #[error("Your previous request to create the named bucket succeeded and you already own it")]
    BucketAlreadyExists,

    // error returned for a negative actual size.
    #[error("Invalid Decompressed Size")]
    InvalidDecompressedSize,

    // error returned in IAM subsystem when user doesn't exist.
    #[error("Specified user does not exist")]
    NoSuchUser,

    // error returned when service account is not found
    #[error("Specified service account does not exist")]
    NoSuchServiceAccount,

    // error returned in IAM subsystem when groups doesn't exist.
    #[error("Specified group does not exist")]
    NoSuchGroup,

    // error returned in IAM subsystem when a non-empty group needs to be
    // deleted.
    #[error("Specified group is not empty - cannot remove it")]
    GroupNotEmpty,

    // error returned in IAM subsystem when policy doesn't exist.
    #[error("Specified canned policy does not exist")]
    NoSuchPolicy,

    // error returned in IAM subsystem when an external users systems is configured.
    #[error("Specified IAM action is not allowed with LDAP configuration")]
    IAMActionNotAllowed,

    // error returned in IAM subsystem when IAM sub-system is still being initialized.
    #[error("IAM sub-system is being initialized, please try again")]
    IAMNotInitialized,

    // error returned when access is denied.
    #[error("Do not have enough permissions to access this resource")]
    AccessDenied,

    // error returned when object is locked.
    #[error("Object is WORM protected and cannot be overwritten or deleted")]
    LockedObject,

    // error returned when upload id not found
    #[error("Specified Upload ID is not found")]
    UploadIDNotFound,
}
