mod types;

use std::borrow::Cow;
use std::path::Path;

use path_absolutize::Absolutize;
pub use types::*;

use crate::endpoint::Endpoint;
use crate::errors::{StorageError, TypedError};
use crate::{config, utils};

const NULL_VERSION_ID: &str = "null";
const BLOCK_SIZE_SMALL: usize = 128 * utils::KIB; // Default r/w block size for smaller objects.
const BLOCK_SIZE_LARGE: usize = 2 * utils::MIB; // Default r/w block size for larger objects.
const BLOCK_SIZE_REALLY_LARGE: usize = 4 * utils::MIB; // Default write block size for objects per shard >= 64MiB

// On regular files bigger than this;
const READ_AHEAD_SIZE: usize = 16 << 20;
// Read this many buffers ahead.
const READ_AHEAD_BUFFERS: usize = 4;
// Size of each buffer.
const READ_AHEAD_BUF_SIZE: usize = 1 << 20;

// Really large streams threshold per shard.
const REALLY_LARGE_FILE_THRESHOLD: usize = 64 * utils::MIB; // Optimized for HDDs

// Small file threshold below which data accompanies metadata from storage layer.
// For hardrives it is possible to set this to a lower value to avoid any
// spike in latency. But currently we are simply keeping it optimal for SSDs.
const SMALL_FILE_THRESHOLD: usize = 128 * utils::KIB; // Optimized for NVMe/SSDs

// XL metadata file carries per object metadata.
const XL_STORAGE_FORMAT_FILE: &str = "xl.meta";

pub(super) struct XlStorage {
    disk_path: String,
    endpoint: Endpoint,

    global_sync: bool,

    root_disk: bool,

    disk_id: String,

    // Indexes, will be -1 until assigned a set.
    pool_index: isize,
    set_index: isize,
    disk_index: isize,

    format_last_check: Option<utils::DateTime>,
}

impl XlStorage {
    pub(super) async fn new(endpoint: Endpoint) -> anyhow::Result<Self> {
        let path = get_valid_path(endpoint.url.path())?;
        let path = path.to_str().ok_or_else(|| StorageError::Unexpected)?;

        let root_disk = if std::env::var("HULK_CI_CD").is_ok() {
            true
        } else {
            let mut root_disk =
                crate::disk::is_root_disk(path.as_ref(), crate::globals::SLASH_SEPARATOR)?;
            if !root_disk {
                if let Ok(root_disk_size) = std::env::var(config::ENV_ROOT_DISK_THRESHOLD_SIZE) {
                    let info = crate::disk::get_info(path).await?;
                    let size = byte_unit::Byte::from_str(&root_disk_size)?;
                    root_disk = info.total <= size.get_bytes();
                }
            }
            root_disk
        };

        let xl = XlStorage {
            disk_path: path.to_owned(),
            endpoint,
            global_sync: std::env::var(config::ENV_FS_OSYNC)
                .as_ref()
                .map_or_else(|_| config::ENABLE_ON, |s| s.as_str())
                == config::ENABLE_ON,
            root_disk,
            disk_id: "".to_string(),
            pool_index: -1,
            set_index: -1,
            disk_index: -1,
            format_last_check: None,
        };

        Ok(xl)
    }
}

pub fn check_path_length(path_name: &str) -> anyhow::Result<(), StorageError> {
    // Apple OS X path length is limited to 1016.
    if cfg!(macos) && path_name.len() > 1016 {
        return Err(StorageError::FileNameTooLong);
    }

    // Disallow more than 1024 characters on windows, there
    // are no known name_max limits on Windows.
    if cfg!(windows) && path_name.len() > 1024 {
        return Err(StorageError::FileNameTooLong);
    }

    // On Unix we reject paths if they are just '.', '..' or '/'.
    if path_name == "." || path_name == ".." || path_name == crate::globals::SLASH_SEPARATOR {
        return Err(StorageError::FileAccessDenied);
    }

    // Check each path segment length is > 255 on all Unix
    // platforms, look for this value as NAME_MAX in
    // /usr/include/linux/limits.h
    let mut count = 0;
    for p in path_name.chars() {
        match p {
            '/' => {
                count = 0;
            }
            '\\' => {
                if cfg!(windows) {
                    count = 0;
                }
            }
            _ => {
                count += 1;
                if count > 255 {
                    return Err(StorageError::FileNameTooLong);
                }
            }
        }
    }

    Ok(())
}

pub fn get_valid_path(path: &str) -> anyhow::Result<Cow<Path>> {
    if path.is_empty() {
        return Err(TypedError::InvalidArgument.into());
    }

    // Disallow relative paths, figure out absolute paths.
    use path_absolutize::Absolutize;
    let path = std::path::Path::new(path).absolutize()?;

    use std::io::ErrorKind;
    match std::fs::metadata(path.as_ref()) {
        Err(err) => {
            if err.kind() != ErrorKind::NotFound {
                return Err(err.into());
            } else {
                // Path not found, create it.
                let _ = std::fs::create_dir_all(path.as_ref())?;
            }
        }
        Ok(meta) => {
            if !meta.is_dir() {
                return Err(StorageError::DiskNotDir.into());
            }
        }
    }

    Ok(path)
}
