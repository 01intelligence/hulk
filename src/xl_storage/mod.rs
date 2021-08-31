mod format_utils;
mod format_v2;
mod openoptions_ext;
mod types;
mod with_check;
use std::fs::Metadata;
use std::io::{Error, ErrorKind, SeekFrom};

pub use format_utils::*;
use futures_util::{ready, FutureExt};
use lazy_static::lazy_static;
use path_absolutize::Absolutize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
pub use types::*;
pub use with_check::*;

use crate::admin::TraceType::Storage;
use crate::endpoint::Endpoint;
use crate::errors::{AsError, StorageError, TypedError};
use crate::fs::{
    check_path_length, err_dir_not_empty, err_invalid_arg, err_io, err_is_dir, err_not_dir,
    err_not_found, err_permission, err_too_many_files, err_too_many_symlinks, AlignedWriter, File,
    OpenOptionsDirectIo,
};
use crate::globals::Guard;
use crate::pool::{TypedPool, TypedPoolGuard};
use crate::prelude::*;
use crate::storage::FileInfo;
use crate::utils::{BufGuard, Path, PathBuf};
use crate::xl_storage::openoptions_ext::{OpenOptionsNoAtime, OpenOptionsSync};
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

const XL_POOL_SMALL_MAX_SIZE: usize = 1024 * 32;
const XL_POOL_LARGE_MAX_SIZE: usize = 1024 * 2;
const XL_POOL_REALLY_LARGE_MAX_SIZE: usize = 1024;

type SmallAlignedBlock = fs::SizedAlignedBlock<BLOCK_SIZE_SMALL>;
type LargeAlignedBlock = fs::SizedAlignedBlock<BLOCK_SIZE_LARGE>;
type ReallyLargeAlignedBlock = fs::SizedAlignedBlock<BLOCK_SIZE_REALLY_LARGE>;

lazy_static! {
    pub static ref XL_POOL_SMALL: Arc<TypedPool<SmallAlignedBlock>> =
        Arc::new(TypedPool::new(XL_POOL_SMALL_MAX_SIZE));
    static ref XL_POOL_LARGE: Arc<TypedPool<LargeAlignedBlock>> =
        Arc::new(TypedPool::new(XL_POOL_LARGE_MAX_SIZE));
    static ref XL_POOL_REALLY_LARGE: Arc<TypedPool<ReallyLargeAlignedBlock>> =
        Arc::new(TypedPool::new(XL_POOL_REALLY_LARGE_MAX_SIZE));
}

impl<const SIZE: usize> utils::BufGuard for TypedPoolGuard<'static, fs::SizedAlignedBlock<SIZE>> {
    fn buf(&self) -> &[u8] {
        &***self
    }
}

impl<const SIZE: usize> utils::BufGuardMut
    for TypedPoolGuard<'static, fs::SizedAlignedBlock<SIZE>>
{
    fn buf_mut(&mut self) -> &mut [u8] {
        &mut ***self
    }
}

// Storage backed by a disk.
pub struct XlStorage {
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

    disk_info_cache: Option<Box<dyn utils::TimedValueGetter<crate::storage::DiskInfo>>>,
}

unsafe impl Send for XlStorage {}
unsafe impl Sync for XlStorage {}

impl XlStorage {
    pub fn is_online(&self) -> bool {
        true
    }

    pub fn last_conn(&self) -> utils::DateTime {
        utils::MIN_DATETIME
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

    pub fn set_disk_id(&mut self, _id: String) {
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

    pub fn healing(&self) -> Option<crate::storage::HealingTracker> {
        todo!()
    }

    fn build_disk_info_cache(&mut self) {
        let this = utils::SendRawPtr::new(self as *mut Self);
        let disk_info_cache = move || async move {
            // Safety: lifetime is bounded by `self`.
            let this = unsafe { this.to().as_mut().unwrap() };
            let info = fs::get_disk_info(&this.disk_path).await?;

            let mut disk_id = None;
            let mut healing = false;
            match this.get_disk_id() {
                Ok(id) => {
                    disk_id = Some(id.to_owned());
                }
                Err(err) => {
                    if err.is_error(&StorageError::UnformattedDisk) {
                        // If we found an unformatted disk then
                        // healing is automatically true.
                        healing = true;
                    } else {
                        // Check if the disk is being healed .
                        healing = this.healing().is_some();
                    }
                }
            };

            Ok(crate::storage::DiskInfo {
                total: info.total,
                free: info.free,
                used: info.used,
                used_inodes: info.files - info.ffree,
                free_inodes: info.ffree,
                fs_type: info.fs_type,
                root_disk: this.root_disk,
                healing,
                endpoint: this.endpoint.to_string(),
                mount_path: this.disk_path.to_owned(),
                id: disk_id.unwrap_or_default(),
                metrics: None,
                error: None,
            })
        };
        self.disk_info_cache = Some(Box::new(utils::TimedValue::new(None, disk_info_cache)));
    }

    pub async fn disk_info(&self) -> anyhow::Result<crate::storage::DiskInfo> {
        self.disk_info_cache.as_ref().unwrap().get().await
    }

    pub(super) async fn new(endpoint: Endpoint) -> anyhow::Result<Self> {
        let path = get_valid_path(endpoint.path()).await?;
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

        let mut xl = XlStorage {
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
            disk_info_cache: None,
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
        utils::rng_seed_now().fill(&mut *aligned_buf);
        let _ = file.write_all(aligned_buf.as_ref()).await?;
        drop(file);
        let _ = fs::remove(&tmp_file).await;

        xl.build_disk_info_cache();

        Ok(xl)
    }

    pub(super) async fn make_volume(&self, volume: &str) -> anyhow::Result<()> {
        if !is_valid_volume_name(volume) {
            return Err(TypedError::InvalidArgument.into());
        }
        let volume_dir = self.get_volume_dir(volume)?;

        match fs::access(&volume_dir).await {
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
                    created: utils::MIN_DATETIME,
                })
            })
            .collect())
    }

    pub async fn stat_volume(&self, volume: &str) -> anyhow::Result<storage::VolInfo> {
        let volume_dir = self.get_volume_dir(volume)?;

        let meta = match fs::metadata(volume_dir).await {
            Err(err) => {
                return if err_not_found(&err) {
                    Err(StorageError::VolumeNotFound.into())
                } else if err_permission(&err) {
                    Err(StorageError::DiskAccessDenied.into())
                } else if err_io(&err) {
                    Err(StorageError::FaultyDisk.into())
                } else {
                    Err(err.into())
                }
            }
            Ok(meta) => meta,
        };

        Ok(storage::VolInfo {
            name: volume.to_owned(),
            created: meta.created_at(),
        })
    }

    pub async fn delete_volume(&self, volume: &str, force_delete: bool) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(volume)?;

        match if force_delete {
            fs::reliable_rename(
                volume_dir,
                Path::new(&self.disk_path)
                    .join(crate::object::SYSTEM_META_TMP_DELETED_BUCKET)
                    .join(uuid::Uuid::new_v4().to_string()),
            )
            .await
        } else {
            fs::remove(volume_dir)
                .await
                .map_err(|err| anyhow::Error::from(err))
        } {
            Err(err) => {
                return if let Some(ierr) = err.as_error::<std::io::Error>() {
                    if err_not_found(ierr) {
                        Err(StorageError::VolumeNotFound.into())
                    } else if err_dir_not_empty(ierr) {
                        Err(StorageError::VolumeNotEmpty.into())
                    } else if err_permission(ierr) {
                        Err(StorageError::DiskAccessDenied.into())
                    } else if err_io(ierr) {
                        Err(StorageError::FaultyDisk.into())
                    } else {
                        Err(err)
                    }
                } else {
                    Err(err)
                };
            }
            Ok(()) => Ok(()),
        }
    }

    pub async fn list_dir(
        &self,
        volume: &str,
        dir_path: &str,
        count: usize,
    ) -> anyhow::Result<Vec<String>> {
        let volume_dir = self.get_volume_dir(volume)?;

        let dir_path = crate::object::path_join(&[&volume_dir, dir_path]);
        match if count > 0 {
            fs::read_dir_entries(dir_path).await
        } else {
            fs::read_dir_entries_n(dir_path, count).await
        } {
            Err(err) => {
                if err_not_found(&err) {
                    if let Err(err) = fs::access(volume_dir).await {
                        if err_not_found(&err) {
                            return Err(StorageError::VolumeNotFound.into());
                        } else if err_io(&err) {
                            return Err(StorageError::FaultyDisk.into());
                        }
                    }
                }
                return Err(err.into());
            }
            Ok(entries) => Ok(entries),
        }
    }

    pub async fn read_all(&self, volume: &str, path: &str) -> anyhow::Result<Vec<u8>> {
        let volume_dir = self.get_volume_dir(volume)?;
        let file_path = crate::object::path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;
        let require_direct_io = &globals::GLOBALS.storage_class.guard().dma
            == crate::config::storageclass::DMA_READ_WRITE;
        read_all_data(&volume_dir, &file_path, require_direct_io).await
    }

    pub async fn delete_version(
        &self,
        volume: &str,
        path: &str,
        file: &storage::FileInfo,
        force_delete_marker: bool,
    ) -> anyhow::Result<()> {
        if path.ends_with(globals::SLASH_SEPARATOR) {
            return self.delete(volume, path, false).await;
        }
        let buf = match self
            .read_all(
                volume,
                &crate::object::path_join(&[path, XL_STORAGE_FORMAT_FILE]),
            )
            .await
        {
            Err(err) => {
                if let Some(&StorageError::FileNotFound) = err.as_error::<StorageError>() {
                    return Err(err);
                }
                if file.deleted && force_delete_marker {}
                if !file.version_id.is_empty() {
                    return Err(StorageError::FileVersionNotFound.into());
                }
                return Err(StorageError::FileNotFound.into());
            }
            Ok(buf) => buf,
        };

        if buf.is_empty() {
            if !file.version_id.is_empty() {
                return Err(StorageError::FileVersionNotFound.into());
            }
            return Err(StorageError::FileNotFound.into());
        }

        let volume_dir = self.get_volume_dir(volume)?;

        todo!()
    }

    pub async fn delete(&self, volume: &str, path: &str, recursive: bool) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(volume)?;
        if let Err(err) = fs::access(&volume_dir).await {
            return if err_not_found(&err) {
                Err(StorageError::VolumeNotFound.into())
            } else if err_permission(&err) {
                Err(StorageError::VolumeAccessDenied.into())
            } else if err_io(&err) {
                Err(StorageError::FaultyDisk.into())
            } else {
                Err(err.into())
            };
        }

        let file_path = crate::object::path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;

        // Delete file, and also delete parent directory if it's empty.
        Self::delete_file(self.disk_path.clone(), volume_dir, file_path, recursive).await
    }

    fn delete_file(
        disk_path: String,
        base_path: String,
        delete_path: String,
        recursive: bool,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + Sync>> {
        Box::pin(async move {
            if base_path.is_empty() || delete_path.is_empty() {
                return Ok(());
            }

            let is_dir = delete_path.ends_with(globals::SLASH_SEPARATOR);
            let base_path = Path::new(&base_path).clean();
            let delete_path = Path::new(&delete_path).clean();
            if !delete_path.starts_with(&base_path) || &delete_path == &base_path {
                return Ok(());
            }

            if let Err(err) = if recursive {
                fs::reliable_rename(
                    &delete_path,
                    Path::new(&disk_path)
                        .join(crate::object::SYSTEM_META_TMP_DELETED_BUCKET)
                        .join(uuid::Uuid::new_v4().to_string()),
                )
                .await
            } else {
                fs::remove(&delete_path)
                    .await
                    .map_err(|err| anyhow::Error::from(err))
            } {
                return if let Some(ierr) = err.as_error::<std::io::Error>() {
                    if err_not_found(ierr) {
                        Ok(())
                    } else if err_dir_not_empty(ierr) {
                        if is_dir {
                            Err(StorageError::FileNotFound.into())
                        } else {
                            Ok(())
                        }
                    } else if err_permission(ierr) {
                        Err(StorageError::FileAccessDenied.into())
                    } else if err_io(ierr) {
                        Err(StorageError::FaultyDisk.into())
                    } else {
                        Err(err)
                    }
                } else {
                    Err(err)
                };
            }

            if let Some(delete_path) = delete_path.parent() {
                // Delete parent directory obviously not recursively. Errors for
                // parent directories shouldn't trickle down.
                let _ = Self::delete_file(
                    disk_path,
                    base_path.to_string(),
                    delete_path.to_string(),
                    recursive,
                )
                .await;
            }

            Ok(())
        })
    }

    pub async fn rename_data(
        &self,
        src_volume: &str,
        src_path: &str,
        fi: FileInfo,
        dest_volume: &str,
        dest_path: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn rename_file(
        &self,
        src_volume: &str,
        src_path: &str,
        dest_volume: &str,
        dest_path: &str,
    ) -> anyhow::Result<()> {
        let src_volume_dir = self.get_volume_dir(src_volume)?;
        let dest_volume_dir = self.get_volume_dir(dest_volume)?;

        if let Err(err) = fs::access(&src_volume_dir).await {
            return if err_not_found(&err) {
                Err(StorageError::VolumeNotFound.into())
            } else if err_io(&err) {
                Err(StorageError::FaultyDisk.into())
            } else {
                Err(err.into())
            };
        }

        if let Err(err) = fs::access(&dest_volume_dir).await {
            return if err_not_found(&err) {
                Err(StorageError::VolumeNotFound.into())
            } else if err_io(&err) {
                Err(StorageError::FaultyDisk.into())
            } else {
                Err(err.into())
            };
        }

        let src_is_dir = crate::object::is_dir(src_path);
        let dest_is_dir = crate::object::is_dir(dest_path);
        if (src_is_dir && !dest_is_dir) || (!src_is_dir && dest_is_dir) {
            return Err(StorageError::FileAccessDenied.into());
        }
        let src_file_path = crate::object::path_join(&[&src_volume_dir, src_path]);
        check_path_length(&src_file_path)?;
        let dest_file_path = crate::object::path_join(&[&dest_volume_dir, dest_path]);
        check_path_length(&dest_file_path)?;
        if src_is_dir {
            // If the src is directory, we expect the dest to be non-existent but we
            // still need to allow overwriting an empty directory.
            match fs::metadata(&dest_file_path).await {
                Err(err) => {
                    if err_io(&err) {
                        return Err(StorageError::FaultyDisk.into());
                    } else if !err_not_found(&err) {
                        return Err(err.into());
                    }
                }
                Ok(dest_meta) => {
                    if !dest_meta.is_dir() {
                        return Err(StorageError::FileAccessDenied.into());
                    }
                    if let Err(err) = fs::remove(&dest_file_path).await {
                        return if err_dir_not_empty(&err) {
                            Err(StorageError::FileAccessDenied.into())
                        } else {
                            Err(err.into())
                        };
                    }
                }
            };
        }

        fs::reliable_rename(&src_file_path, &dest_file_path).await?;

        // Remove parent dir of the src file if empty.
        if let Some(src_parent_dir) = Path::new(&src_file_path).parent() {
            let _ = Self::delete_file(
                self.disk_path.clone(),
                src_volume_dir,
                src_parent_dir.to_string(),
                false,
            )
            .await;
        }

        Ok(())
    }

    pub async fn verify_file(&self, volume: &str, path: &str, fi: &FileInfo) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(volume)?;

        if let Err(err) = fs::access(&volume_dir).await {
            return if err_not_found(&err) {
                Err(StorageError::VolumeNotFound.into())
            } else if err_permission(&err) {
                Err(StorageError::VolumeAccessDenied.into())
            } else if err_io(&err) {
                Err(StorageError::FaultyDisk.into())
            } else {
                Err(err.into())
            };
        }

        assert!(fi.erasure.is_some());
        let erasure = fi.erasure.as_ref().unwrap();

        for part in &fi.parts {
            let checksum_info = erasure.get_checksum_info(part.number).unwrap();
            let part_path = crate::object::path_join(&[
                &volume_dir,
                path,
                &fi.data_dir,
                &format!("part.{}", part.number),
            ]);
            let file = fs::OpenOptions::new().read(true).open(&part_path).await?;
            let file_size = file.metadata().await?.len();
            crate::bitrot::bitrot_verify(
                file,
                file_size,
                erasure.shard_file_size(part.size),
                checksum_info.algorithm,
                &checksum_info.hash,
                erasure.shard_size(),
            )
            .await?; // TODO: logging error
        }
        todo!();

        Ok(())
    }

    pub async fn create_file_writer(
        &self,
        volume: &str,
        path: &str,
        file_size: Option<u64>,
    ) -> anyhow::Result<Box<dyn AsyncWrite + Unpin>> {
        let volume_dir = self.get_volume_dir(volume)?;
        let file_path = crate::object::path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;

        fs::reliable_mkdir_all(&volume_dir, 0o777).await?;

        let mut pool_guard = (None, None);
        let writer = if file_size.is_some() && file_size.unwrap() <= SMALL_FILE_THRESHOLD as u64 {
            // For small files, we simply write them as O_DSYNC and not O_DIRECT
            // to avoid the complexities of aligned I/O.
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .sync()
                .open(&file_path)
                .await
            {
                Err(err) => Err(err),
                Ok(file) => Ok((Some(file), None)),
            }
        } else {
            match fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open_direct_io(&file_path)
                .await
            {
                Err(err) => Err(err),
                Ok(file) => {
                    let mut buf;
                    if file_size.is_some()
                        && file_size.unwrap() >= REALLY_LARGE_FILE_THRESHOLD as u64
                    {
                        // Really large files.
                        pool_guard.0 = Some(XL_POOL_REALLY_LARGE.get().await?);
                        buf = &mut ***pool_guard.0.as_mut().unwrap();
                    } else {
                        // Large files.
                        pool_guard.1 = Some(XL_POOL_LARGE.get().await?);
                        buf = &mut ***pool_guard.1.as_mut().unwrap();
                    }
                    // Safety: lifetime of `buf` is controlled by `pool_guard`.
                    let buf: &'static mut [u8] =
                        unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr(), buf.len()) };
                    let file = file.into_std().await;
                    // Aligned write.
                    Ok((None, Some(fs::AlignedWriter::new(file, buf, file_size))))
                }
            }
        };

        let mut writer = match writer {
            Err(err) => {
                let err = if err_is_dir(&err) {
                    StorageError::IsNotRegular.into()
                } else if err_permission(&err) {
                    StorageError::FileAccessDenied.into()
                } else if err_io(&err) {
                    StorageError::FaultyDisk.into()
                } else if err_too_many_files(&err) {
                    StorageError::TooManyOpenFiles.into()
                } else {
                    err.into()
                };
                return Err(err);
            }
            Ok(w) => w,
        };

        debug_assert!(writer.0.is_some() || writer.1.is_some());

        let volume = volume.to_owned();
        let w = FileWriter {
            writer,
            pool_guard,
            file_size,
            written: 0,
            sync: None,
            has_err: false,
            cleanup: async move {
                // If error, cleanup system meta tmp volume dir.
                if &volume == crate::object::SYSTEM_META_TMP_BUCKET {
                    let _ = fs::reliable_remove_all(&volume_dir).await;
                }
            }
            .boxed_local(),
        };
        Ok(Box::new(w))
    }

    pub async fn read_file_reader(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        size: u64,
    ) -> anyhow::Result<Box<dyn AsyncRead + Unpin + Send>> {
        let volume_dir = self.get_volume_dir(volume)?;
        let file_path = crate::object::path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;

        let mut open_options = fs::OpenOptions::new();
        open_options.read(true).no_atime();
        let mut file = match if offset == 0
            && &globals::GLOBALS.storage_class.guard().dma
                == crate::config::storageclass::DMA_READ_WRITE
        {
            // O_DIRECT only supported if `offset` is 0.
            open_options.open_direct_io(&file_path).await
        } else {
            open_options.open(&file_path).await
        } {
            Ok(file) => file,
            Err(err) => {
                let err = if err_not_found(&err) {
                    if let Err(err) = fs::access(volume_dir).await {
                        if err_not_found(&err) {
                            return Err(StorageError::VolumeNotFound.into());
                        }
                    }
                    StorageError::FileNotFound.into()
                } else if err_permission(&err) {
                    StorageError::FileAccessDenied.into()
                } else if err_is_dir(&err) {
                    StorageError::IsNotRegular.into()
                } else if err_io(&err) {
                    StorageError::FaultyDisk.into()
                } else if err_too_many_files(&err) {
                    StorageError::TooManyOpenFiles.into()
                } else if err_invalid_arg(&err) {
                    StorageError::UnsupportedDisk.into()
                } else {
                    err.into()
                };
                return Err(err);
            }
        };

        let meta = file.metadata().await?;
        if !meta.is_file() {
            return Err(StorageError::IsNotRegular.into());
        }

        if offset == 0
            && &globals::GLOBALS.storage_class.guard().dma
                == crate::config::storageclass::DMA_READ_WRITE
        {
            struct PoolGuard(
                Option<TypedPoolGuard<'static, SmallAlignedBlock>>,
                Option<TypedPoolGuard<'static, LargeAlignedBlock>>,
            );
            impl utils::BufGuard for PoolGuard {
                fn buf(&self) -> &[u8] {
                    if let Some(guard) = &self.0 {
                        guard.buf()
                    } else {
                        self.1.as_ref().unwrap().buf()
                    }
                }
            }
            impl utils::BufGuardMut for PoolGuard {
                fn buf_mut(&mut self) -> &mut [u8] {
                    if let Some(guard) = &mut self.0 {
                        guard.buf_mut()
                    } else {
                        self.1.as_mut().unwrap().buf_mut()
                    }
                }
            }

            let pool_guard = if size <= SMALL_FILE_THRESHOLD as u64 {
                PoolGuard(Some(XL_POOL_SMALL.get().await?), None)
            } else {
                PoolGuard(None, Some(XL_POOL_LARGE.get().await?))
            };

            let reader = fs::AlignedReader::new(file.into_std().await, pool_guard);
            let reader = reader.take(size);
            return Ok(Box::new(reader));
        }

        if offset > 0 {
            file.seek(SeekFrom::Start(offset)).await?;
        }

        let mut reader = file.take(size);

        // Add read-ahead to big reads.
        if size >= READ_AHEAD_SIZE as u64 {
            let reader =
                crate::io::ReadAhead::new(reader, READ_AHEAD_BUFFERS, READ_AHEAD_BUF_SIZE).await;
            return Ok(Box::new(reader));
        }

        // Just add a small 64k buffer.
        let reader = tokio::io::BufReader::with_capacity(64 << 10, reader);
        Ok(Box::new(reader))
    }

    fn get_volume_dir(&self, volume: &str) -> anyhow::Result<String> {
        match volume {
            "" | "." | ".." => Err(StorageError::VolumeNotFound.into()),
            _ => Ok(crate::object::path_join(&[&self.disk_path, volume])),
        }
    }
}
async fn read_all_data(
    volume_dir: &str,
    file_path: &str,
    require_direct_io: bool,
) -> anyhow::Result<Vec<u8>> {
    let mut r = match if require_direct_io {
        match fs::OpenOptions::new()
            .read(true)
            .no_atime()
            .open_direct_io(file_path)
            .await
        {
            Ok(r) => {
                let guard = XL_POOL_SMALL.get().await?;
                Ok(Box::pin(crate::io::BufReader::new(r, guard))
                    as Pin<Box<dyn tokio::io::AsyncRead>>)
            }
            Err(err) => Err(err),
        }
    } else {
        fs::OpenOptions::new()
            .read(true)
            .no_atime()
            .open(file_path)
            .await
            .map(|r| Box::pin(r) as Pin<Box<dyn tokio::io::AsyncRead>>)
    } {
        Err(err) => {
            if err_not_found(&err) {
                if let Err(err) = fs::access(volume_dir).await {
                    if err_not_found(&err) {
                        return Err(StorageError::VolumeNotFound.into());
                    }
                }
                return Err(StorageError::FileNotFound.into());
            } else if err_permission(&err) {
                return Err(StorageError::FileAccessDenied.into());
            } else if err_not_dir(&err) || err_is_dir(&err) {
                return Err(StorageError::FileNotFound.into());
            } else if err_io(&err) {
                return Err(StorageError::FaultyDisk.into());
            } else if err_too_many_files(&err) {
                return Err(StorageError::TooManyOpenFiles.into());
            } else if err_invalid_arg(&err) {
                if let Ok(meta) = fs::metadata(file_path).await {
                    if meta.is_dir() {
                        return Err(StorageError::FileNotFound.into());
                    }
                }
                return Err(StorageError::UnsupportedDisk.into());
            }
            return Err(err.into());
        }
        Ok(r) => r,
    };
    let mut buf = Vec::new();
    r.read_to_end(&mut buf).await?;
    Ok(buf)
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

struct FileWriter {
    writer: (Option<File>, Option<AlignedWriter<'static>>),
    pool_guard: (
        Option<TypedPoolGuard<'static, ReallyLargeAlignedBlock>>,
        Option<TypedPoolGuard<'static, LargeAlignedBlock>>,
    ),
    file_size: Option<u64>,
    written: u64,
    sync: Option<futures_util::future::LocalBoxFuture<'static, std::io::Result<()>>>,
    has_err: bool,
    cleanup: futures_util::future::LocalBoxFuture<'static, ()>,
}

impl AsyncWrite for FileWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        match ready!(if let Some(writer) = &mut this.writer.0 {
            Pin::new(writer).poll_write(cx, buf)
        } else {
            Pin::new(&mut this.writer.1.as_mut().unwrap()).poll_write(cx, buf)
        }) {
            Ok(n) => {
                this.written += n as u64;
                Poll::Ready(Ok(n))
            }
            Err(err) => {
                this.has_err = true;
                Poll::Ready(Err(err))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();

        if let Some(file_size) = this.file_size {
            if this.written < file_size {
                return Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::Other,
                    StorageError::LessData,
                )));
            } else if this.written > file_size {
                return Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::Other,
                    StorageError::MoreData,
                )));
            };
        }

        // Sync only data not metadata.
        if this.sync.is_none() {
            if let Some(writer) = this.writer.0.take() {
                this.sync = Some(async move { writer.sync_data().await }.boxed_local());
            } else if let Some(writer) = this.writer.1.take() {
                this.sync = Some(async move { writer.sync_data().await }.boxed_local());
            }
        }
        if let Some(sync) = &mut this.sync {
            ready!(sync.poll_unpin(cx))?;
        }

        if this.has_err {
            ready!(this.cleanup.poll_unpin(cx));
        }

        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}
