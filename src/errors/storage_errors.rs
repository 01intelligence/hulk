use std::convert::TryFrom;

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StorageError {
    #[error("unexpected error, please report this issue at https://github.com/hulk/hulk/issues")]
    Unexpected,

    #[error("corrupted backend format, specified disk mount has unexpected previous content")]
    CorruptedFormat,

    #[error("unformatted disk found")]
    UnformattedDisk,

    #[error("inconsistent disk found")]
    InconsistentDisk,

    #[error("disk does not support O_DIRECT")]
    UnsupportedDisk,

    #[error("disk path full")]
    DiskFull,

    #[error("disk is not directory or mountpoint")]
    DiskNotDir,

    #[error("disk not found")]
    DiskNotFound,

    #[error("remote disk is faulty")]
    FaultyRemoteDisk,

    #[error("disk is faulty")]
    FaultyDisk,

    #[error("disk access denied")]
    DiskAccessDenied,

    #[error("file not found")]
    FileNotFound,

    #[error("file version not found")]
    FileVersionNotFound,

    #[error("too many open files, please increase 'ulimit -n'")]
    TooManyOpenFiles,

    #[error("file name too long")]
    FileNameTooLong,

    #[error("volume already exists")]
    VolumeExists,

    #[error("not of regular file type")]
    IsNotRegular,

    #[error("path not found")]
    PathNotFound,

    #[error("volume not found")]
    VolumeNotFound,

    #[error("volume is not empty")]
    VolumeNotEmpty,

    #[error("volume access denied")]
    VolumeAccessDenied,

    #[error("file access denied")]
    FileAccessDenied,

    #[error("file is corrupted")]
    FileCorrupt,

    #[error("parent is a file")]
    FileParentIsFile,

    // verification is empty or invalid.
    #[error("bit-rot hash algorithm is invalid")]
    BitrotHashAlgoInvalid,

    #[error(
        "Rename across devices ({0} -> {1}) not allowed, please fix your backend configuration"
    )]
    CrossDeviceLink(String, String),

    #[error("The disk size is less than 900MiB threshold")]
    MinDiskSize,

    #[error("less data available than what was requested")]
    LessData,

    #[error("more data was sent than what was advertised")]
    MoreData,

    #[error("done for now")]
    DoneForNow,

    #[error("skip this file")]
    SkipFile,

    #[error("io error: {0}")]
    IoError(std::io::Error),
}

pub const BASE_STORAGE_ERRORS: [StorageError; 3] = [
    StorageError::DiskNotFound,
    StorageError::FaultyDisk,
    StorageError::FaultyRemoteDisk,
];

impl TryFrom<std::io::Error> for StorageError {
    type Error = std::io::Error;

    fn try_from(err: std::io::Error) -> Result<Self, Self::Error> {
        use crate::fs;
        if fs::err_not_found(&err) || fs::err_not_dir(&err) || fs::err_is_dir(&err) {
            return Ok(StorageError::FileNotFound);
        } else if fs::err_permission(&err) {
            return Ok(StorageError::FileAccessDenied);
        } else if fs::err_too_many_files(&err) {
            return Ok(StorageError::TooManyOpenFiles);
        } else if fs::err_io(&err) {
            return Ok(StorageError::FaultyDisk);
        } else if fs::err_invalid_arg(&err) {
            return Ok(StorageError::UnsupportedDisk);
        } else if fs::err_no_space(&err) {
            return Ok(StorageError::DiskFull);
        }
        Err(err)
    }
}
