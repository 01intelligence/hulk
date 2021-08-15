mod format_utils;
mod types;

use std::borrow::Cow;
use std::io::Error;

pub use format_utils::*;
use lazy_static::lazy_static;
use path_absolutize::Absolutize;
use tokio::io::AsyncWriteExt;
pub use types::*;

use crate::endpoint::Endpoint;
use crate::errors::{AsError, StorageError, TypedError};
use crate::fs::{check_path_length, err_io, err_not_found, err_permission, OpenOptionsDirectIo};
use crate::utils::Path;
use crate::{config, fs, globals, storage, utils};

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

// Storage backed by a disk.
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
    pub fn is_online(&self) -> bool {
        true
    }

    pub fn last_conn(&self) -> utils::DateTime {
        utils::min_datetime()
    }

    pub fn is_local(&self) -> bool {
        true
    }

    pub fn hostname(&self) -> &str {
        self.endpoint.host()
    }

    pub fn endpoint(&self) -> &crate::endpoint::Endpoint {
        &self.endpoint
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn get_disk_id(&self) -> anyhow::Result<&str> {
        todo!()
    }

    pub fn set_disk_id(&mut self, id: String) {
        // Nothing to do.
    }

    pub fn get_disk_location(&self) -> (isize, isize, isize) {
        if self.pool_index < 0 || self.set_index < 0 || self.disk_index < 0 {
            // If unset, see if we can locate it.
            return get_xl_disk_loc(&self.disk_id);
        }
        (self.pool_index, self.set_index, self.disk_index)
    }

    pub fn set_disk_location(&mut self, pool_idx: isize, set_idx: isize, disk_idx: isize) {
        self.pool_index = pool_idx;
        self.set_index = set_idx;
        self.disk_index = disk_idx;
    }

    pub(super) async fn new(endpoint: Endpoint) -> anyhow::Result<Self> {
        let path = get_valid_path(endpoint.url.path()).await?;
        let path = path.to_str().ok_or_else(|| StorageError::Unexpected)?;

        let root_disk = if std::env::var("HULK_CI_CD").is_ok() {
            true
        } else {
            let mut root_disk = fs::is_root_disk(path.as_ref(), crate::globals::SLASH_SEPARATOR)?;
            // If for some reason we couldn't detect the
            // root disk use - HULK_ROOTDISK_THRESHOLD_SIZE
            // to figure out if the disk is root disk or not.
            if !root_disk {
                if let Ok(root_disk_size) = std::env::var(config::ENV_ROOT_DISK_THRESHOLD_SIZE) {
                    let info = fs::get_info(path).await?;
                    let size = byte_unit::Byte::from_str(&root_disk_size)?;
                    // Size of the disk is less than the threshold or
                    // equal to the size of the disk at path, treat
                    // such disks as root_disks and reject them.
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

        // Check if backend is writable and supports O_DIRECT
        use utils::Rng;
        let rnd = utils::rng_seed_now().gen::<[u8; 8]>();
        let tmp_file = format!(".writable-check-{}.tmp", hex::encode(rnd));
        let tmp_file =
            crate::object::path_join(&[&xl.disk_path, globals::SYSTEM_RESERVED_BUCKET, &tmp_file]);
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open_direct_io(&tmp_file)
            .await?;
        let mut aligned_buf = fs::AlignedBlock::new(4096);
        utils::rng_seed_now().fill(aligned_buf.as_mut());
        let _ = file.write_all(aligned_buf.as_ref()).await?;
        drop(file);
        let _ = fs::remove(&tmp_file).await;

        Ok(xl)
    }

    pub(super) async fn make_volume(&self, volume: &str) -> anyhow::Result<()> {
        if !is_valid_volume_name(volume) {
            return Err(TypedError::InvalidArgument.into());
        }
        let volume_dir = self.get_volume_dir(volume)?;

        match fs::access(&volume_dir) {
            Ok(_) => Err(StorageError::VolumeExists.into()),
            Err(mut err) => {
                let mut any_err: anyhow::Error;
                // If volume does not exist, we proceed to create.
                if err_not_found(&err) {
                    // Make a volume entry, with mode 0777 mkdir honors system umask.
                    match fs::reliable_mkdir_all(volume_dir, 0o777).await {
                        Ok(_) => return Ok(()),
                        Err(err) => {
                            any_err = err;
                        }
                    }
                } else {
                    any_err = err.into();
                }
                let err_ref = if let Some(err_ref) = any_err.as_error::<std::io::Error>() {
                    err_ref
                } else {
                    return Err(any_err.into());
                };
                if err_permission(err_ref) {
                    Err(StorageError::DiskAccessDenied.into())
                } else if err_io(err_ref) {
                    Err(StorageError::FaultyDisk.into())
                } else {
                    Err(any_err.into())
                }
            }
        }
    }

    pub async fn make_volumes(&self, volumes: &[&str]) -> anyhow::Result<()> {
        for volume in volumes {
            if let Err(err) = self.make_volume(volume).await {
                if let Some(err) = err.as_error::<StorageError>() {
                    if let &StorageError::VolumeExists = err {
                        continue;
                    }
                }
                return Err(err);
            }
        }
        Ok(())
    }

    pub async fn list_volumes(&self) -> anyhow::Result<Vec<storage::VolInfo>> {
        let _ = check_path_length(&self.disk_path)?;
        let entries = fs::read_dir_entries(&self.disk_path).await?;
        Ok(entries
            .into_iter()
            .filter_map(|entry| {
                if !entry.ends_with(crate::globals::SLASH_SEPARATOR)
                    || !is_valid_volume_name(&entry)
                {
                    // Skip if entry is neither a directory not a valid volume name.
                    return None;
                }
                Some(storage::VolInfo {
                    name: entry,
                    created: utils::min_datetime(),
                })
            })
            .collect())
    }

    fn get_volume_dir(&self, volume: &str) -> anyhow::Result<String> {
        match volume {
            "" | "." | ".." => Err(StorageError::VolumeNotFound.into()),
            _ => Ok(crate::object::path_join(&[&self.disk_path, volume])),
        }
    }
}

lazy_static! {
    static ref RESERVED_CHARS: Vec<char> = r#"\:*?\"<>|"#.chars().collect();
}

fn is_valid_volume_name(volume: &str) -> bool {
    if volume.len() < 3 {
        return false;
    }
    if cfg!(windows) {
        // Volname shouldn't have reserved characters in Windows.
        return !volume.contains(&RESERVED_CHARS[..]);
    }
    true
}

pub async fn get_valid_path(path: &str) -> anyhow::Result<Cow<'_, Path>> {
    if path.is_empty() {
        return Err(TypedError::InvalidArgument.into());
    }

    // Disallow relative paths, figure out absolute paths.
    use crate::utils::PathAbsolutize;
    let path = crate::utils::Path::new(path).absolutize()?;

    use std::io::ErrorKind;
    match fs::metadata(path.as_ref()).await {
        Err(err) => {
            if err.kind() != ErrorKind::NotFound {
                return Err(err.into());
            } else {
                // Path not found, create it.
                let _ = fs::reliable_mkdir_all(path.as_ref(), 0o777).await?;
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
