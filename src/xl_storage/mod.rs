mod format_utils;
mod format_v2;
mod types;
mod with_check;
use std::fs::Metadata;
use std::io::{Error, ErrorKind, SeekFrom};

pub use format_utils::*;
pub use format_v2::*;
use futures_util::{ready, FutureExt, StreamExt};
use lazy_static::lazy_static;
use path_absolutize::Absolutize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::RwLock;
pub use types::*;
pub use with_check::*;

use crate::admin::TraceType::Storage;
use crate::bitrot::BitrotVerifier;
use crate::endpoint::Endpoint;
use crate::errors::{AsError, StorageError, TypedError};
use crate::fs::{
    check_path_length, err_dir_not_empty, err_invalid_arg, err_io, err_is_dir, err_not_dir,
    err_not_found, err_permission, err_too_many_files, err_too_many_symlinks, AlignedWriter, File,
    OpenOptionsDirectIo, OpenOptionsNoAtime, OpenOptionsSync, SameFile,
};
use crate::globals::Guard;
use crate::io::{AsyncReadAt, AsyncReadFull};
use crate::metacache::MetaCacheEntry;
use crate::object::{self, path_ensure_dir, path_is_dir, path_join};
use crate::pool::{TypedPool, TypedPoolGuard};
use crate::prelude::*;
use crate::storage::FileInfo;
use crate::utils::{BufGuard, DateTimeExt, Path, PathBuf, TimedValueUpdateFnResult};
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

    // Indexes, will be -1 until assigned a set.
    pool_index: isize,
    set_index: isize,
    disk_index: isize,

    meta_cache: RwLock<Option<XlStorageMeta>>,

    disk_info_cache: utils::TimedValue<crate::storage::DiskInfo>,
}

struct XlStorageMeta {
    disk_id: String,
    format_meta: fs::Metadata,
    format_last_check: utils::DateTime,
}

unsafe impl Send for XlStorage {}
unsafe impl Sync for XlStorage {}

impl XlStorage {
    pub fn is_online(&self) -> bool {
        true
    }

    pub fn last_conn(&self) -> utils::DateTime {
        utils::DateTime::zero()
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

    pub async fn get_disk_id(&self) -> anyhow::Result<String> {
        // Read lock.
        let meta_cache = self.meta_cache.read().await;
        let mut last_check = None;
        if let Some(meta) = &*meta_cache {
            // If cached `disk_id` is less than 1s old.
            if meta.format_last_check.elapsed() <= utils::seconds(1) {
                return Ok(meta.disk_id.clone());
            }
            last_check = Some(meta.format_last_check);
        }
        drop(meta_cache);

        // Write lock.
        let mut meta_cache = self.meta_cache.write().await;

        // If somebody else has updated `disk_id`.
        if let Some(meta) = &*meta_cache {
            if let Some(last_check) = last_check {
                if meta.format_last_check > last_check {
                    return Ok(meta.disk_id.clone());
                }
            }
        }

        let format_file = path_join(&[
            &self.disk_path,
            object::SYSTEM_META_BUCKET,
            crate::format::FORMAT_CONFIG_FILE,
        ]);

        let meta = match fs::metadata(&format_file).await {
            Ok(meta) => meta,
            Err(err) => {
                return if err_not_found(&err) {
                    match fs::access(&self.disk_path).await {
                        Ok(_) => Err(StorageError::UnformattedDisk.into()),
                        Err(err) => {
                            if err_not_found(&err) {
                                Err(StorageError::DiskNotFound.into())
                            } else if err_permission(&err) {
                                Err(StorageError::DiskAccessDenied.into())
                            } else {
                                Err(StorageError::CorruptedFormat.into())
                            }
                        }
                    }
                } else if err_permission(&err) {
                    Err(StorageError::DiskAccessDenied.into())
                } else {
                    Err(StorageError::CorruptedFormat.into())
                };
            }
        };

        // If the format file has not changed, just return the cached `disk_id`.
        if let Some(meta_cache) = &mut *meta_cache {
            if meta.is_same_file(&meta_cache.format_meta) {
                meta_cache.format_last_check = utils::now(); // cache check time
                return Ok(meta_cache.disk_id.clone());
            }
        }

        let content = fs::read_file(&format_file).await?;
        let format: crate::format::FormatErasureV3 = serde_json::from_slice(&content)?;
        let disk_id = format.erasure.this;

        // Cache it anyhow.
        meta_cache.insert(XlStorageMeta {
            disk_id: disk_id.clone(),
            format_meta: meta,
            format_last_check: utils::now(),
        });

        Ok(disk_id)
    }

    pub fn set_disk_id(&mut self, _id: String) {
        // Nothing to do.
    }

    pub async fn get_disk_location(&self) -> (isize, isize, isize) {
        if self.pool_index < 0 || self.set_index < 0 || self.disk_index < 0 {
            // If unset, see if we can locate it.
            let meta_cache = self.meta_cache.read().await;
            if let Some(meta) = &*meta_cache {
                return get_xl_disk_loc(&meta.disk_id);
            }
        }
        (self.pool_index, self.set_index, self.disk_index)
    }

    pub fn set_disk_location(&mut self, pool_idx: isize, set_idx: isize, disk_idx: isize) {
        self.pool_index = pool_idx;
        self.set_index = set_idx;
        self.disk_index = disk_idx;
    }

    pub async fn healing(&self) -> Option<crate::storage::HealingTracker> {
        todo!()
    }

    pub async fn disk_info(&self) -> anyhow::Result<crate::storage::DiskInfo> {
        let get_disk_info = async move || {
            let info = fs::get_disk_info(&self.disk_path).await?;

            let mut disk_id = None;
            let mut healing = false;
            match self.get_disk_id().await {
                Ok(id) => {
                    disk_id = Some(id);
                }
                Err(err) => {
                    if err.is_error(&StorageError::UnformattedDisk) {
                        // If we found an unformatted disk then
                        // healing is automatically true.
                        healing = true;
                    } else {
                        // Check if the disk is being healed .
                        healing = self.healing().await.is_some();
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
                root_disk: self.root_disk,
                healing,
                endpoint: self.endpoint.to_string(),
                mount_path: self.disk_path.to_owned(),
                id: disk_id.unwrap_or_default(),
                metrics: None,
                error: None,
            })
        };

        self.disk_info_cache.get(Some(get_disk_info)).await
    }

    pub async fn new_local(path: &str) -> anyhow::Result<Self> {
        let result = url::Url::from_file_path(path);
        match result {
            Ok(u) => {
                let endpoint = Endpoint::Url(u, true);
                XlStorage::new(endpoint).await
            }
            Err(_) => Err(TypedError::InvalidArgument.into()),
        }
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

        let xl = XlStorage {
            disk_path: path.to_owned(),
            endpoint,
            global_sync: std::env::var(config::ENV_FS_OSYNC)
                .as_ref()
                .map_or_else(|_| config::ENABLE_ON, |s| s.as_str())
                == config::ENABLE_ON,
            root_disk,
            pool_index: -1,
            set_index: -1,
            disk_index: -1,
            meta_cache: RwLock::new(None),
            disk_info_cache: utils::TimedValue::new(None, None),
        };

        // Check if backend is writable and supports O_DIRECT
        use utils::Rng;
        let rnd = utils::rng_seed_now().gen::<[u8; 8]>();
        let tmp_file = format!(".writable-check-{}.tmp", hex::encode(rnd));
        let tmp_dir = path_join(&[&xl.disk_path, globals::SYSTEM_RESERVED_BUCKET]);
        let tmp_file = path_join(&[&xl.disk_path, globals::SYSTEM_RESERVED_BUCKET, &tmp_file]);
        fs::reliable_mkdir_all(&tmp_dir, 0o777).await?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open_direct_io(&tmp_file)
            .await?;
        let mut aligned_buf = fs::AlignedBlock::new(4096);
        utils::rng_seed_now().fill(&mut *aligned_buf);
        let _ = file.write_all(aligned_buf.as_ref()).await?;
        drop(file);
        let _ = fs::remove_all(&tmp_dir).await;

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
        let entries = fs::read_dir_entries(&self.disk_path).await;
        match entries {
            Ok(entries) => Ok(entries
                .into_iter()
                .filter_map(|entry| {
                    if !entry.ends_with(crate::globals::SLASH_SEPARATOR)
                        || !is_valid_volume_name(&entry)
                    {
                        // Skip if entry is neither a directory not a valid volume name.
                        return None;
                    }
                    Some(storage::VolInfo {
                        name: entry
                            .trim_end_matches(crate::globals::SLASH_SEPARATOR)
                            .to_string(),
                        created: utils::DateTime::zero(),
                    })
                })
                .collect()),
            Err(err) => match StorageError::try_from(err) {
                Ok(err) => Err(err.into()),
                Err(err) => Err(err.into()),
            },
        }
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

        let dir_path = path_join(&[&volume_dir, dir_path]);
        match if count > 0 {
            fs::read_dir_entries_n(dir_path, count).await
        } else {
            fs::read_dir_entries(dir_path).await
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
                    return Err(StorageError::FileNotFound.into());
                }
                match StorageError::try_from(err) {
                    Ok(err) => Err(err.into()),
                    Err(err) => Err(err.into()),
                }
            }
            Ok(entries) => Ok(entries),
        }
    }

    pub async fn read_version(
        &self,
        volume: &str,
        path: &str,
        version_id: &str,
        read_data: bool,
    ) -> anyhow::Result<FileInfo> {
        let volume_dir = self.get_volume_dir(volume)?;
        let buf = self
            .read_all(volume, &path_join(&[path, XL_STORAGE_FORMAT_FILE]))
            .await?;

        if buf.is_empty() {
            return if !version_id.is_empty() {
                Err(StorageError::FileVersionNotFound.into())
            } else {
                Err(StorageError::FileNotFound.into())
            };
        }

        let mut fi = get_file_info(&buf, volume, path, version_id, read_data)?;

        if read_data {
            if !fi.data.is_empty() || fi.size == 0 {
                if !fi.data.is_empty() {
                    let key = globals::RESERVED_METADATA_PREFIX_LOWER.to_owned() + "inline-data";
                    let _ = fi.metadata.entry(key).or_insert("true".to_owned());
                }
                return Ok(fi);
            }

            // Reading data for small objects when:
            // - object has not yet transitioned
            // - object size is small
            // - object has maximum of 1 parts
            if fi.transition_status.is_empty()
                && fi.data_dir.is_empty()
                && fi.size <= SMALL_FILE_THRESHOLD as u64
                && fi.parts.len() == 1
            {
                let require_direct_io = &globals::GLOBALS.storage_class.guard().dma
                    == crate::config::storageclass::DMA_READ_WRITE;
                let part_path = format!("part.{}", fi.parts[0].number);
                fi.data = read_all_data(
                    &volume_dir,
                    &path_join(&[&volume_dir, path, &fi.data_dir, &part_path]),
                    require_direct_io,
                )
                .await?;
            }
        }

        Ok(fi)
    }

    pub async fn read_all(&self, volume: &str, path: &str) -> anyhow::Result<Vec<u8>> {
        let volume_dir = self.get_volume_dir(volume)?;
        let file_path = path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;
        let require_direct_io = &globals::GLOBALS.storage_class.guard().dma
            == crate::config::storageclass::DMA_READ_WRITE;
        read_all_data(&volume_dir, &file_path, require_direct_io).await
    }

    pub async fn delete_versions(
        &self,
        volume: &str,
        versions: &[&FileInfo],
    ) -> Vec<anyhow::Result<()>> {
        let mut errs = Vec::with_capacity(versions.len());
        for version in versions {
            let ret = self
                .delete_version(volume, &version.name, version, false)
                .await;
            errs.push(ret);
        }
        errs
    }

    pub async fn delete_version(
        &self,
        volume: &str,
        path: &str,
        fi: &storage::FileInfo,
        force_delete_marker: bool,
    ) -> anyhow::Result<()> {
        if path.ends_with(globals::SLASH_SEPARATOR) {
            return self.delete(volume, path, false).await;
        }
        let mut buf = match self
            .read_all(volume, &path_join(&[path, XL_STORAGE_FORMAT_FILE]))
            .await
        {
            Err(err) => {
                if let Some(&StorageError::FileNotFound) = err.as_error::<StorageError>() {
                    return Err(err);
                }
                if fi.deleted && force_delete_marker {
                    return self.write_metadata(volume, path, fi).await;
                }
                if !fi.version_id.is_empty() {
                    return Err(StorageError::FileVersionNotFound.into());
                }
                return Err(StorageError::FileNotFound.into());
            }
            Ok(buf) => buf,
        };

        if buf.is_empty() {
            if !fi.version_id.is_empty() {
                return Err(StorageError::FileVersionNotFound.into());
            }
            return Err(StorageError::FileNotFound.into());
        }

        let volume_dir = self.get_volume_dir(volume)?;

        if !is_xl2_v1_format(&buf) {
            // Delete the meta file, if there are no more versions the
            // top level parent is automatically removed.
            return Self::delete_file(
                self.disk_path.clone(),
                volume_dir.clone(),
                path_join(&[&volume_dir, path]),
                true,
            )
            .await;
        }

        let mut xl_meta = XlMetaV2::load_with_data(&buf)?;
        let (data_dir, last_version) = xl_meta.delete_version(fi)?;

        if !data_dir.is_empty() {
            let mut version_id: &str = &fi.version_id;
            if version_id.is_empty() {
                version_id = NULL_VERSION_ID;
            }
            let _ = xl_meta.data.remove(version_id);
            let _ = xl_meta.data.remove(&data_dir);
            let file_path = path_join(&[&volume_dir, path, &data_dir]);
            check_path_length(&file_path)?;

            fs::reliable_rename(
                &file_path,
                path_join(&[
                    &self.disk_path,
                    object::SYSTEM_META_TMP_DELETED_BUCKET,
                    &uuid::Uuid::new_v4().to_string(),
                ]),
            )
            .await?;
        }

        if !last_version {
            buf = xl_meta.dump()?;
            return self
                .write_all(volume, &path_join(&[path, XL_STORAGE_FORMAT_FILE]), &buf)
                .await;
        }

        // Move everything to trash.
        let dir_path = path_ensure_dir(&path_join(&[&volume_dir, path])).into_owned();
        check_path_length(&dir_path)?;
        let ret = fs::reliable_rename(
            &dir_path,
            path_join(&[
                &self.disk_path,
                object::SYSTEM_META_TMP_DELETED_BUCKET,
                &uuid::Uuid::new_v4().to_string(),
            ]),
        )
        .await;

        // Delete parents if needed.
        let dir_path = path_ensure_dir(
            Path::new(&path_join(&[&volume_dir, path]))
                .parent()
                .unwrap()
                .as_str(),
        )
        .into_owned();
        if &dir_path != &path_ensure_dir(&volume_dir) {
            let _ = Self::delete_file(self.disk_path.clone(), volume_dir, dir_path, false).await;
        }

        ret
    }

    pub async fn append_file(&self, volume: &str, path: &str, buf: &[u8]) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(volume)?;
        if let Err(err) = fs::access(&volume_dir).await {
            if err_permission(&err) {
                return Err(StorageError::VolumeAccessDenied.into());
            } else if err_not_found(&err) {
                return Err(StorageError::VolumeNotFound.into());
            } else {
                return match StorageError::try_from(err) {
                    Ok(err) => Err(err.into()),
                    Err(err) => Err(err.into()),
                };
            }
        }

        let file_path = path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;
        if let Some((dir_path, _)) = file_path.rsplit_once(globals::SLASH_SEPARATOR) {
            fs::reliable_mkdir_all(dir_path, 0o777).await?;
        } else {
            fs::reliable_mkdir_all(&volume_dir, 0o777).await?;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .sync()
            .open(file_path)
            .await;
        if let Err(err) = file {
            if err_permission(&err) {
                return Err(StorageError::IsNotRegular.into());
            } else {
                return match StorageError::try_from(err) {
                    Ok(err) => Err(err.into()),
                    Err(err) => Err(err.into()),
                };
            }
        };
        let mut file = file.unwrap();
        if let Err(err) = file.write_all(buf).await {
            return match StorageError::try_from(err) {
                Ok(err) => Err(err.into()),
                Err(err) => Err(err.into()),
            };
        }

        Ok(())
    }

    pub async fn check_parts(&self, volume: &str, path: &str, fi: &FileInfo) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(volume)?;
        if let Err(err) = fs::access(&volume_dir).await {
            return match StorageError::try_from(err) {
                Ok(err) => Err(err.into()),
                Err(err) => Err(err.into()),
            };
        }

        for part in &fi.parts {
            let part_path = path_join(&[path, &fi.data_dir, &format!("part.{}", part.number)]);
            let file_path = path_join(&[&volume_dir, &part_path]);
            check_path_length(&file_path)?;
            let meta = fs::metadata(&file_path).await?;
            if meta.is_dir() {
                return Err(StorageError::FileNotFound.into());
            }
            // Check if shard is truncated.
            if meta.len()
                < fi.erasure
                    .as_ref()
                    .map(|e| e.shard_file_size(part.size))
                    .unwrap_or_default()
            {
                return Err(StorageError::FileCorrupt.into());
            }
        }

        Ok(())
    }

    pub async fn check_file(&self, volume: &str, path: &str) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(volume)?;

        Self::check_file_inner(volume_dir, Some(path.to_owned())).await
    }

    fn check_file_inner(
        volume_dir: String,
        path: Option<String>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + Sync>> {
        Box::pin(async move {
            let path = match path {
                None => return Err(StorageError::PathNotFound.into()),
                Some(path) => path,
            };
            if &path == "." || &path == globals::SLASH_SEPARATOR {
                return Err(StorageError::PathNotFound.into());
            }

            let file_path = path_join(&[&volume_dir, &path, XL_STORAGE_FORMAT_FILE]);
            check_path_length(&file_path)?;
            match fs::metadata(&file_path).await {
                Err(_) => {
                    Self::check_file_inner(
                        volume_dir,
                        Path::new(&path).parent().map(|p| p.to_string()),
                    )
                    .await
                }
                Ok(meta) => {
                    if !meta.is_file() {
                        Err(StorageError::FileNotFound.into())
                    } else {
                        Ok(())
                    }
                }
            }
        })
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

        let file_path = path_join(&[&volume_dir, path]);
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

    pub async fn update_metadata(
        &self,
        volume: &str,
        path: &str,
        fi: &FileInfo,
    ) -> anyhow::Result<()> {
        let path = path_join(&[path, XL_STORAGE_FORMAT_FILE]);
        let mut buf = match self.read_all(volume, &path).await {
            Ok(buf) => buf,
            Err(err) => {
                return if err.is_error(&StorageError::FileNotFound) && !fi.version_id.is_empty() {
                    Err(StorageError::FileVersionNotFound.into())
                } else {
                    Err(err)
                }
            }
        };

        if !is_xl2_v1_format(&buf) {
            return Err(StorageError::FileVersionNotFound.into());
        }

        let mut xl_meta = XlMetaV2::load_with_data(&buf)?;
        xl_meta.update_version(fi)?;

        let buf = xl_meta.dump()?;
        self.write_all(volume, &path, &buf).await
    }

    pub async fn write_metadata(
        &self,
        volume: &str,
        path: &str,
        fi: &FileInfo,
    ) -> anyhow::Result<()> {
        let path = path_join(&[path, XL_STORAGE_FORMAT_FILE]);
        let mut buf = self.read_all(volume, &path).await?;

        let mut xl_meta = if !is_xl2_v1_format(&buf) {
            XlMetaV2::default()
        } else {
            XlMetaV2::load_with_data(&buf)?
        };
        xl_meta.add_version(fi)?;
        buf = xl_meta.dump()?;

        self.write_all(volume, &path, &buf).await
    }

    pub async fn write_all(&self, volume: &str, path: &str, data: &[u8]) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(volume)?;
        let file_path = path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;

        fs::reliable_mkdir_all(&volume_dir, 0o777).await?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .sync()
            .open(&file_path)
            .await?;

        file.write_all(data).await?;

        Ok(())
    }

    pub async fn rename_data(
        &self,
        src_volume: &str,
        src_path: &str,
        fi: FileInfo,
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

        let src_file_path = path_join(&[&src_volume_dir, src_path, XL_STORAGE_FORMAT_FILE]);
        check_path_length(&src_file_path)?;
        let dest_file_path = path_join(&[&dest_volume_dir, dest_path, XL_STORAGE_FORMAT_FILE]);
        check_path_length(&dest_file_path)?;

        let data_dir = path_ensure_dir(&fi.data_dir);
        let mut data_path = None;
        if !data_dir.is_empty() {
            let src_data_path =
                path_ensure_dir(&path_join(&[&src_volume_dir, src_path, &data_dir])).into_owned();
            let dest_data_path = path_join(&[&dest_volume_dir, dest_path, &data_dir]);
            data_path = Some((src_data_path, dest_data_path));
        }

        let dest_buf = fs::read_file(&dest_file_path).await?;
        let mut xl_meta = if !dest_buf.is_empty() {
            XlMetaV2::load_with_data(&dest_buf)?
        } else {
            XlMetaV2::default()
        };

        let mut old_dest_data_path = None;
        if fi.version_id.is_empty() {
            if let Ok(ofi) = xl_meta.to_file_info(dest_volume, dest_path, NULL_VERSION_ID) {
                if !ofi.deleted {
                    if xl_meta.shared_data_dir_str_count(NULL_VERSION_ID, &ofi.data_dir) == 0 {
                        old_dest_data_path =
                            Some(path_join(&[&dest_volume_dir, dest_path, &ofi.data_dir]));
                        let _ = xl_meta.data.remove(NULL_VERSION_ID);
                        let _ = xl_meta.data.remove(&ofi.data_dir);
                    }
                }
            }
        }

        xl_meta.add_version(&fi)?;

        let dest_buf = xl_meta.dump()?;

        if let Some((src_data_path, dest_data_path)) = data_path {
            self.write_all(
                src_volume,
                &path_join(&[src_path, XL_STORAGE_FORMAT_FILE]),
                &dest_buf,
            )
            .await?;

            if fi.data.is_empty() && fi.size > 0 {
                let _ = fs::reliable_rename(
                    &dest_data_path,
                    &path_join(&[
                        &self.disk_path,
                        crate::object::SYSTEM_META_TMP_DELETED_BUCKET,
                        &uuid::Uuid::new_v4().to_string(),
                    ]),
                )
                .await;
                if let Err(err) = fs::reliable_rename(&src_data_path, &dest_data_path).await {
                    Self::delete_file(
                        self.disk_path.clone(),
                        dest_volume_dir,
                        dest_file_path,
                        false,
                    )
                    .await;
                    return Err(err);
                }
            }

            if let Err(err) = fs::reliable_rename(&src_file_path, &dest_file_path).await {
                Self::delete_file(
                    self.disk_path.clone(),
                    dest_volume_dir,
                    dest_file_path,
                    false,
                )
                .await;
                return Err(err);
            }

            if let Some(old_dest_data_path) = old_dest_data_path {
                let _ = fs::reliable_rename(
                    &old_dest_data_path,
                    &path_join(&[
                        &self.disk_path,
                        crate::object::SYSTEM_META_TMP_DELETED_BUCKET,
                        &uuid::Uuid::new_v4().to_string(),
                    ]),
                )
                .await;
            }
        } else {
            if let Err(err) = self
                .write_all(
                    dest_volume,
                    &path_join(&[dest_path, XL_STORAGE_FORMAT_FILE]),
                    &dest_buf,
                )
                .await
            {
                Self::delete_file(
                    self.disk_path.clone(),
                    dest_volume_dir,
                    dest_file_path,
                    false,
                )
                .await;
                return Err(err);
            }
        }

        let _ = fs::remove(Path::new(&src_file_path).parent().unwrap()).await;

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

        let src_is_dir = path_is_dir(src_path);
        let dest_is_dir = path_is_dir(dest_path);
        if (src_is_dir && !dest_is_dir) || (!src_is_dir && dest_is_dir) {
            return Err(StorageError::FileAccessDenied.into());
        }
        let src_file_path = path_join(&[&src_volume_dir, src_path]);
        check_path_length(&src_file_path)?;
        let dest_file_path = path_join(&[&dest_volume_dir, dest_path]);
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
            let part_path = path_join(&[
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

        Ok(())
    }

    pub async fn create_file_writer(
        &self,
        volume: &str,
        path: &str,
        file_size: Option<u64>,
    ) -> anyhow::Result<Box<dyn AsyncWrite + Unpin>> {
        let volume_dir = self.get_volume_dir(volume)?;
        let file_path = path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;

        fs::reliable_mkdir_all(&volume_dir, 0o777).await?;

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
                Ok(file) => Ok(FileWriterEnum::Left(file)),
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
                    let buf_guard = if file_size.is_some()
                        && file_size.unwrap() >= REALLY_LARGE_FILE_THRESHOLD as u64
                    {
                        // Really large files.
                        utils::EitherGuard::Left(XL_POOL_REALLY_LARGE.get().await?)
                    } else {
                        // Large files.
                        utils::EitherGuard::Right(XL_POOL_LARGE.get().await?)
                    };
                    let file = file.into_std().await;
                    // Aligned write.
                    Ok(FileWriterEnum::Right(fs::AlignedWriter::new(
                        file, buf_guard, file_size,
                    )))
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

        let volume = volume.to_owned();
        let w = FileWriter {
            writer,
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

    pub async fn read_file(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        buf: &mut [u8],
        verifier: Option<crate::bitrot::BitrotVerifier>,
    ) -> anyhow::Result<u64> {
        let volume_dir = self.get_volume_dir(volume)?;

        if let Err(err) = fs::access(&volume_dir).await {
            return if err_not_found(&err) {
                Err(StorageError::VolumeNotFound.into())
            } else if err_io(&err) {
                Err(StorageError::FaultyDisk.into())
            } else if err_permission(&err) {
                Err(StorageError::FileAccessDenied.into())
            } else {
                Err(err.into())
            };
        }

        let file_path = path_join(&[&volume_dir, path]);
        check_path_length(&file_path)?;

        let mut file = fs::OpenOptions::new().read(true).open(&file_path).await;
        if let Err(err) = file {
            return if err_not_found(&err) {
                Err(StorageError::FileNotFound.into())
            } else if err_permission(&err) {
                Err(StorageError::IsNotRegular.into())
            } else {
                match StorageError::try_from(err) {
                    Ok(err) => Err(err.into()),
                    Err(err) => Err(err.into()),
                }
            };
        };
        let mut file = file.unwrap();
        let meta = match file.metadata().await {
            Ok(meta) => meta,
            Err(err) => {
                return match StorageError::try_from(err) {
                    Ok(err) => Err(err.into()),
                    Err(err) => Err(err.into()),
                }
            }
        };

        let verifier = match verifier {
            None => {
                return match file.read_at(buf, offset).await {
                    Ok(n) => Ok(n as u64),
                    Err(err) => Err(err.into()),
                };
            }
            Some(v) => v,
        };

        let mut h = verifier.algorithm.hasher();
        let mut reader = file.take(offset);
        if let Err(err) = tokio::io::copy(&mut reader, &mut h).await {
            return match StorageError::try_from(err) {
                Ok(err) => Err(err.into()),
                Err(err) => Err(err.into()),
            };
        };

        let mut file = reader.into_inner();
        let n = match file.read_full(buf).await {
            Ok(n) => n,
            Err(err) => {
                return match StorageError::try_from(err) {
                    Ok(err) => Err(err.into()),
                    Err(err) => Err(err.into()),
                };
            }
        };
        if n != buf.len() {
            let err: std::io::Error = ErrorKind::UnexpectedEof.into();
            return Err(err.into());
        }

        h.append(buf);
        let _ = tokio::io::copy(&mut file, &mut h).await?;

        if h.finish() != &verifier.hash[..] {
            return Err(StorageError::FileCorrupt.into());
        }

        Ok(buf.len() as u64)
    }

    pub async fn read_file_reader(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        size: u64,
    ) -> anyhow::Result<Box<dyn AsyncRead + Unpin + Send>> {
        let volume_dir = self.get_volume_dir(volume)?;
        let file_path = path_join(&[&volume_dir, path]);
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

    pub async fn walk_dir<W: AsyncWrite + Unpin + Send + 'static>(
        &self,
        opts: crate::metacache::WalkDirOptions,
        w: W,
    ) -> anyhow::Result<()> {
        let volume_dir = self.get_volume_dir(&opts.bucket)?;

        if let Err(err) = fs::access(&volume_dir).await {
            return if err_not_found(&err) {
                Err(StorageError::VolumeNotFound.into())
            } else if err_io(&err) {
                Err(StorageError::FaultyDisk.into())
            } else {
                Err(err.into())
            };
        }

        let mut w = crate::metacache::MetaCacheWriter::new(crate::io::AsyncWriteStdWriter::new(w));
        let (handle, tx) = w.write_sender()?;

        let closure = async move || {
            if let Some(base_dir) = opts.base_dir.strip_suffix(globals::SLASH_SEPARATOR) {
                match XlMetaV2::load_from_file(&path_join(&[
                    &volume_dir,
                    &(base_dir.to_owned() + globals::GLOBAL_DIR_SUFFIX),
                    XL_STORAGE_FORMAT_FILE,
                ]))
                .await
                {
                    Ok(xl_meta) => {
                        let xl_meta = xl_meta.dump()?; // TODO
                        let permit = tx.reserve().await?;
                        permit.send(MetaCacheEntry::new(
                            opts.base_dir.clone(),
                            Arc::new(xl_meta),
                        ));
                    }
                    Err(_) => {
                        match fs::metadata(&path_join(&[
                            &volume_dir,
                            &opts.base_dir,
                            XL_STORAGE_FORMAT_FILE,
                        ]))
                        .await
                        {
                            Ok(meta) => {
                                if meta.is_file() {
                                    return Err(StorageError::FileNotFound.into());
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
            }

            let forward = &opts.forward_to as &str;
            let cur_dir = Cow::Borrowed(&opts.base_dir as &str);

            self.walk_dir_inner(&opts, &volume_dir, &tx, forward, cur_dir)
                .await
        };

        closure().await?;
        let mut w = handle.await??;
        w.close()
    }

    fn walk_dir_inner<'a, 'b: 'a, 'c: 'a, 'd: 'a, 'e: 'a, 'f: 'a, 'g: 'a>(
        &'b self,
        opts: &'c crate::metacache::WalkDirOptions,
        volume_dir: &'d str,
        tx: &'e tokio::sync::mpsc::Sender<MetaCacheEntry>,
        forward: &'f str,
        cur_dir: Cow<'g, str>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + Sync + 'a>>
where {
        Box::pin(async move {
            let mut entries = match self.list_dir(&opts.bucket, cur_dir.as_ref(), 0).await {
                Err(err) => {
                    if opts.report_not_found && cur_dir.as_ref() == &opts.base_dir {
                        if err.is_error(&StorageError::FileNotFound) {
                            return Err(StorageError::FileNotFound.into());
                        }
                    }
                    return Ok(());
                }
                Ok(entries) => entries,
            };
            let mut dir_objects = HashSet::new();
            for entry in entries.iter_mut() {
                if !opts.filter_prefix.is_empty() && !entry.starts_with(&opts.filter_prefix) {
                    continue;
                }
                if !forward.is_empty() && (entry as &str) < forward {
                    continue;
                }
                if entry.ends_with(globals::SLASH_SEPARATOR) {
                    if entry.ends_with(globals::GLOBAL_DIR_SUFFIX_WITH_SLASH) {
                        *entry = entry
                            .strip_suffix(globals::GLOBAL_DIR_SUFFIX_WITH_SLASH)
                            .unwrap()
                            .to_owned()
                            + globals::SLASH_SEPARATOR;
                        // Safety: immutable borrow.
                        dir_objects
                            .insert(unsafe { (entry as *const String).as_ref().unwrap() } as &str);
                        continue;
                    }
                    entry.remove(entry.len() - 1); // remove slash suffix
                    continue;
                }
                entry.clear();
                if entry.ends_with(XL_STORAGE_FORMAT_FILE) {
                    match XlMetaV2::load_from_file(&path_join(&[
                        volume_dir,
                        cur_dir.as_ref(),
                        entry,
                    ]))
                    .await
                    {
                        Err(_) => {
                            continue;
                        }
                        Ok(xl_meta) => {
                            let xl_meta = xl_meta.dump()?; // TODO
                            let name = entry
                                .strip_suffix(XL_STORAGE_FORMAT_FILE)
                                .unwrap()
                                .strip_suffix(globals::SLASH_SEPARATOR)
                                .unwrap();
                            let name = path_join(&[cur_dir.as_ref(), name]);
                            let name = crate::object::decode_dir_object(&name);
                            let permit = tx.reserve().await?;
                            permit.send(MetaCacheEntry::new(name.into_owned(), Arc::new(xl_meta)));
                            return Ok(());
                        }
                    }
                }
            }

            entries.sort_unstable();
            let mut dir_stack = Vec::<String>::with_capacity(5);
            for entry in entries.iter() {
                if entry.is_empty() {
                    continue;
                }
                let mut name = path_join(&[cur_dir.as_ref(), entry]);
                while !dir_stack.is_empty() && dir_stack.last().unwrap() < &name {
                    let pop = dir_stack.pop().unwrap();
                    let permit = tx.reserve().await?;
                    permit.send(MetaCacheEntry::new(pop.clone(), Arc::new(Vec::new())));
                    if opts.recursive {
                        let forward =
                            if !opts.forward_to.is_empty() && opts.forward_to.starts_with(&pop) {
                                opts.forward_to.strip_prefix(&pop).unwrap()
                            } else {
                                ""
                            };
                        let cur_dir = Cow::Owned(pop);
                        self.walk_dir_inner(&opts, volume_dir, tx, forward, cur_dir)
                            .await; // scan next
                    }
                }

                let is_dir_obj = dir_objects.contains(&(entry as &str));
                if is_dir_obj {
                    name.replace_range(
                        name.len() - 1..name.len(),
                        globals::GLOBAL_DIR_SUFFIX_WITH_SLASH,
                    );
                }

                match XlMetaV2::load_from_file(&path_join(&[
                    volume_dir,
                    &name,
                    XL_STORAGE_FORMAT_FILE,
                ]))
                .await
                {
                    Ok(xl_meta) => {
                        let xl_meta = xl_meta.dump()?; // TODO
                        if is_dir_obj {
                            name.replace_range(
                                name.rfind(globals::GLOBAL_DIR_SUFFIX_WITH_SLASH).unwrap()
                                    ..globals::GLOBAL_DIR_SUFFIX_WITH_SLASH.len(),
                                globals::SLASH_SEPARATOR,
                            );
                        }
                        let permit = tx.reserve().await?;
                        permit.send(MetaCacheEntry::new(name, Arc::new(xl_meta)));
                    }
                    Err(err) => {
                        let mut skip = false;
                        if let Some(err) = err.as_error::<std::io::Error>() {
                            if err_not_found(&err) {
                                if !is_dir_obj {
                                    let name = name + globals::SLASH_SEPARATOR;
                                    if !fs::is_dir_empty(&path_join(&[volume_dir, &name])).await {
                                        dir_stack.push(name);
                                    }
                                }
                                skip = true;
                            } else if err_not_dir(&err) {
                                skip = true;
                            }
                        }
                        if !skip {
                            // TODO: log
                        }
                    }
                }
            }

            if !dir_stack.is_empty() {
                let pop = dir_stack.pop().unwrap();
                let permit = tx.reserve().await?;
                permit.send(MetaCacheEntry::new(pop.clone(), Arc::new(Vec::new())));
                if opts.recursive {
                    let forward =
                        if !opts.forward_to.is_empty() && opts.forward_to.starts_with(&pop) {
                            opts.forward_to.strip_prefix(&pop).unwrap()
                        } else {
                            ""
                        };
                    let cur_dir = Cow::Owned(pop);
                    self.walk_dir_inner(&opts, volume_dir, tx, forward, cur_dir)
                        .await; // scan next
                }
            }
            Ok(())
        })
    }

    pub async fn namespace_scanner(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn get_volume_dir(&self, volume: &str) -> anyhow::Result<String> {
        match volume {
            "" | "." | ".." => Err(StorageError::VolumeNotFound.into()),
            _ => Ok(path_join(&[&self.disk_path, volume])),
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
    let buf_size = r.read_to_end(&mut buf).await;
    match buf_size {
        Ok(_) => Ok(buf),
        Err(err) => match StorageError::try_from(err) {
            Ok(err) => Err(err.into()),
            Err(err) => Err(err.into()),
        },
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

#[pin_project::pin_project]
struct FileWriter<G: utils::BufGuardMut + 'static> {
    #[pin]
    writer: FileWriterEnum<G>,
    file_size: Option<u64>,
    written: u64,
    sync: Option<futures_util::future::LocalBoxFuture<'static, std::io::Result<()>>>,
    has_err: bool,
    cleanup: futures_util::future::LocalBoxFuture<'static, ()>,
}

#[pin_project::pin_project(project = FileWriterEnumProj)]
enum FileWriterEnum<G: utils::BufGuardMut> {
    Left(#[pin] File),
    Right(#[pin] AlignedWriter<G>),
    None,
}

impl<G: utils::BufGuardMut + 'static> AsyncWrite for FileWriter<G> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.project();

        match ready!(match this.writer.project() {
            FileWriterEnumProj::Left(w) => w.poll_write(cx, buf),
            FileWriterEnumProj::Right(w) => w.poll_write(cx, buf),
            _ => panic!(), // never go here
        }) {
            Ok(n) => {
                *this.written += n as u64;
                Poll::Ready(Ok(n))
            }
            Err(err) => {
                *this.has_err = true;
                Poll::Ready(Err(err))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.project();

        if let Some(file_size) = *this.file_size {
            if *this.written < file_size {
                return Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::Other,
                    StorageError::LessData,
                )));
            } else if *this.written > file_size {
                return Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::Other,
                    StorageError::MoreData,
                )));
            };
        }

        // Sync only data not metadata.
        if this.sync.is_none() {
            // Safety: sync only once.
            let writer = unsafe { this.writer.get_unchecked_mut() };
            let writer = std::mem::replace(writer, FileWriterEnum::None);
            match writer {
                FileWriterEnum::Left(writer) => {
                    *this.sync = Some(async move { writer.sync_data().await }.boxed_local());
                }
                FileWriterEnum::Right(writer) => {
                    *this.sync = Some(async move { writer.sync_data().await }.boxed_local());
                }
                _ => {}
            }
        }
        if let Some(sync) = this.sync {
            ready!(sync.poll_unpin(cx))?;
        }

        if *this.has_err {
            ready!(this.cleanup.poll_unpin(cx));
        }

        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    #[cfg(not(windows))]
    use std::os::unix::fs::PermissionsExt;

    use async_std::println;
    use bstr::io::BufReadExt;
    use scopeguard::defer;
    use tempfile::tempdir_in;
    use utils::Rng;

    use super::*;
    use crate::bitrot::{BitrotAlgorithm, HighwayBitrotWriter};
    use crate::config::storageclass;
    use crate::endpoint::Endpoint;
    use crate::format::FORMAT_CONFIG_FILE;
    use crate::fs::{
        get_disk_info, is_dir_empty, mkdir_all, read_file, reliable_remove_all, remove_all,
        set_permissions, OpenOptions, OpenOptionsDirectIo, Permissions,
    };
    use crate::object::{SLASH_SEPARATOR, SYSTEM_META_BUCKET};
    use crate::utils::assert::{assert_err, assert_ok};

    // Tests validate volume name.
    #[test]
    fn test_is_valid_volname() {
        let cases: [(&str, bool); 31] = [
            // Cases which should pass the test.
            // passing in valid bucket names.
            ("lol", true),
            ("1-this-is-valid", true),
            ("1-this-too-is-valid-1", true),
            ("this.works.too.1", true),
            ("1234567", true),
            ("123", true),
            ("s3-eu-west-1.amazonaws.com", true),
            ("ideas-are-more-powerful-than-guns", true),
            ("testbucket", true),
            ("1bucket", true),
            ("bucket1", true),
            ("$this-is-not-valid-too", true),
            ("contains-$-dollar", true),
            ("contains-^-carrot", true),
            ("contains-$-dollar", true),
            ("contains-$-dollar", true),
            (".starts-with-a-dot", true),
            ("ends-with-a-dot.", true),
            ("ends-with-a-dash-", true),
            ("-starts-with-a-dash", true),
            ("THIS-BEINGS-WITH-UPPERCASe", true),
            ("tHIS-ENDS-WITH-UPPERCASE", true),
            ("ThisBeginsAndEndsWithUpperCase", true),
            ("una ñina", true),
            (
                "lalalallalallalalalallalallalala-theString-size-is-greater-than-64",
                true,
            ),
            // cases for which test should fail.
            // passing invalid bucket names.
            ("", false),
            (SLASH_SEPARATOR, false),
            ("a", false),
            ("ab", false),
            ("ab/", true),
            ("......", true),
        ];
        for (vol_name, should_pass) in cases.iter() {
            assert_eq!(is_valid_volume_name(vol_name), *should_pass);
        }
    }

    // creates a temp dir and sets up xlStorage layer.
    // returns xlStorage layer, temp dir path to be used for the purpose of tests.
    async fn new_xl_storage_test_setup() -> anyhow::Result<(XlStorageWithCheck, PathBuf)> {
        let disk_path =
            PathBuf::from_path_buf(tempdir_in(".").unwrap().into_path()).expect("utils.PathBuf");

        assert_ok!(
            mkdir_all(disk_path.as_path(), 0o777).await,
            "Unable to create temporary directory. {:?}",
            disk_path
        );

        // Initialize a new xlStorage layer.
        let end_point = assert_ok!(Endpoint::new(disk_path.to_str().unwrap()));

        let storage = assert_ok!(XlStorage::new(end_point).await);

        // Create a sample format.json file
        assert_ok!(storage.write_all(SYSTEM_META_BUCKET, FORMAT_CONFIG_FILE, r#"{"version":"1","format":"xl","id":"592a41c2-b7cc-4130-b883-c4b5cb15965b","xl":{"version":"3","this":"da017d62-70e3-45f1-8a1a-587707e69ad1","sets":[["e07285a6-8c73-4962-89c6-047fb939f803","33b8d431-482d-4376-b63c-626d229f0a29","cff6513a-4439-4dc1-bcaa-56c9e880c352","da017d62-70e3-45f1-8a1a-587707e69ad1","9c9f21d5-1f15-4737-bce6-835faa0d9626","0a59b346-1424-4fc2-9fa2-a2e80541d0c1","7924a3dc-b69a-4971-9a2e-014966d6aebb","4d2b8dd9-4e48-444b-bdca-c89194b26042"]],"distributionAlgo":"CRCMOD"}}"#.as_bytes()).await);
        let mut disk = XlStorageWithCheck::new(storage);
        *disk.disk_id_mut() = Some(String::from("da017d62-70e3-45f1-8a1a-587707e69ad1"));
        Ok((disk, disk_path.clone()))
    }

    // create_perm_denied_file - creates temporary directory and file with
    // path '/mybucket/myobject'
    #[cfg(target_family = "unix")]
    async fn create_perm_denied_file() -> String {
        let mut file_path =
            PathBuf::from_path_buf(tempdir_in(".").unwrap().into_path()).expect("utils.PathBuf");

        assert_ok!(
            mkdir_all(file_path.as_path(), 0o755).await,
            "Unable to create temporary directory. {:?}",
            file_path
        );
        file_path.push("mybucket");
        assert_ok!(
            mkdir_all(file_path.as_path(), 0o755).await,
            "Unable to create temporary directory. {:?}",
            file_path
        );
        file_path.push("myobject");
        #[cfg(windows)]
        let mut object_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .sync()
                .open(file_path.to_str().unwrap())
                .await,
            "Unable to create file. {:?}",
            file_path
        );
        #[cfg(not(windows))]
        let mut object_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .mode(0o400)
                .sync()
                .open(file_path.to_str().unwrap())
                .await,
            "Unable to create file. {:?}",
            file_path
        );
        assert_ok!(
            object_file.write_all(b"").await,
            "Unable to write file. {:?}",
            file_path
        );
        assert!(file_path.pop());
        assert_ok!(
            set_permissions(file_path.as_path(), Permissions::from_mode(0o400)).await,
            "Unable to set directory permissions. {:?}",
            file_path
        );
        assert!(file_path.pop());
        assert_ok!(
            set_permissions(file_path.as_path(), Permissions::from_mode(0o400)).await,
            "Unable to set directory permissions. {:?}",
            file_path
        );

        String::from(file_path.to_str().unwrap())
    }

    // remove_perm_denied_file - removes temporary directory and file with
    // path '/mybucket/myobject'
    #[cfg(target_family = "unix")]
    async fn remove_perm_denied_file(perm_denied_dir: &str) {
        assert_ok!(
            set_permissions(perm_denied_dir, Permissions::from_mode(0o755)).await,
            "Unable to set directory permissions. {:?}",
            perm_denied_dir
        );
        let mybucket_dir = path_join(&[perm_denied_dir, "mybucket"]);
        assert_ok!(
            set_permissions(&mybucket_dir, Permissions::from_mode(0o755)).await,
            "Unable to set directory permissions. {:?}",
            &mybucket_dir
        );
        assert_ok!(remove_all(perm_denied_dir).await);
    }

    // test_xl_storage_read_version - test_xl_storages the functionality
    // implemented by xlStorage ReadVersion storage API.
    #[tokio::test]
    async fn test_xl_storage_read_version() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}
        let xl_meta = read_file("./src/xl_storage/testdata/xl.meta")
            .await
            .unwrap();

        // Create files for the test cases.
        assert_ok!(
            xl_storage.make_volume("exists").await,
            "Unable to create a volume exists."
        );
        assert_ok!(
            xl_storage
                .append_file("exists", "as-directory/as-file/xl.meta", &xl_meta)
                .await,
            "Unable to create a file as-directory/as-file"
        );
        assert_ok!(
            xl_storage
                .append_file("exists", "as-file/xl.meta", &xl_meta)
                .await,
            "Unable to create a file as-file"
        );
        assert_ok!(
            xl_storage
                .append_file("exists", "as-file-parent/xl.meta", &xl_meta)
                .await,
            "Unable to create a file as-file-parent"
        );

        // TestXLStoragecases to validate different conditions for ReadVersion API.
        let cases: [(&str, &str, Option<StorageError>); 6] = [
            // TestXLStorage case - 1.
            // Validate volume does not exist.
            (
                "i-dont-exist",
                "",
                Some(StorageError::VolumeNotFound.into()),
            ),
            // TestXLStorage case - 2.
            // Validate bad condition file does not exist.
            (
                "exists",
                "as-file-not-found",
                Some(StorageError::FileNotFound.into()),
            ),
            // TestXLStorage case - 3.
            // Validate bad condition file exists as prefix/directory and
            // we are attempting to read it.
            (
                "exists",
                "as-directory",
                Some(StorageError::FileNotFound.into()),
            ),
            // TestXLStorage case - 4.
            (
                "exists",
                "as-file-parent/as-file",
                Some(StorageError::FileNotFound.into()),
            ),
            // TestXLStorage case - 5.
            // Validate the good condition file exists and we are able to read it.
            // TODO: meta minor version not match and xl.meta format not match
            ("exists", "as-file", None),
            // TestXLStorage case - 6.
            // TestXLStorage case with invalid volume name.
            ("ab", "as-file", Some(StorageError::VolumeNotFound.into())),
        ];

        // Run through all the test cases and validate for ReadVersion.
        for (volume, path, expected_err) in cases.iter() {
            let result = xl_storage.read_version(volume, path, "", false).await;
            match result {
                Ok(_) => assert_eq!(None, *expected_err),
                Err(err) => {
                    if *expected_err != None {
                        assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string());
                    } else {
                        // TODO: testdata xl.meta is 1.0. now only support 1.2. also xl.meta format not match
                        assert_eq!(err.to_string(), "xl.meta: unknown minor metadata version");
                    }
                }
            }
        }
    }

    // test_xl_storage_read_all - test_xl_storages the functionality
    // implemented by xlStorage ReadAll storage API.
    #[tokio::test]
    async fn test_xl_storage_read_all() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        // Create files for the test cases.
        assert_ok!(
            xl_storage.make_volume("exists").await,
            "Unable to create a volume exists."
        );
        assert_ok!(
            xl_storage
                .append_file("exists", "as-directory/as-file", b"Hello, World")
                .await,
            "Unable to create a file as-directory/as-file"
        );
        assert_ok!(
            xl_storage
                .append_file("exists", "as-file", b"Hello, World")
                .await,
            "Unable to create a file as-file"
        );
        assert_ok!(
            xl_storage
                .append_file("exists", "as-file-parent", b"Hello, World")
                .await,
            "Unable to create a file as-file-parent"
        );

        // TestXLStoragecases to validate different conditions for ReadAll API.
        let cases: [(&str, &str, Option<StorageError>); 6] = [
            // TestXLStorage case - 1.
            // Validate volume does not exist.
            (
                "i-dont-exist",
                "",
                Some(StorageError::VolumeNotFound.into()),
            ),
            // TestXLStorage case - 2.
            // Validate bad condition file does not exist.
            (
                "exists",
                "as-file-not-found",
                Some(StorageError::FileNotFound.into()),
            ),
            // TestXLStorage case - 3.
            // Validate bad condition file exists as prefix/directory and
            // we are attempting to read it.
            (
                "exists",
                "as-directory",
                #[cfg(windows)]
                Some(StorageError::FileAccessDenied.into()),
                #[cfg(not(windows))]
                Some(StorageError::IsNotRegular.into()),
            ),
            // TestXLStorage case - 4.
            (
                "exists",
                "as-file-parent/as-file",
                Some(StorageError::FileNotFound.into()),
            ),
            // TestXLStorage case - 5.
            // Validate the good condition file exists and we are able to read it.
            ("exists", "as-file", None),
            // TestXLStorage case - 6.
            // TestXLStorage case with invalid volume name.
            ("ab", "as-file", Some(StorageError::VolumeNotFound.into())),
        ];

        // Run through all the test cases and validate for ReadAll.
        for (volume, path, expected_err) in cases.iter() {
            println!("volume {} path {}", volume, path);
            let result = xl_storage.read_all(volume, path).await;
            match result {
                Ok(data_read) => {
                    assert_eq!(None, *expected_err);
                    assert_eq!(data_read, b"Hello, World");
                }
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }
    }

    // test_new_xl_storage all the cases handled in xlStorage storage
    // layer initialization.
    #[tokio::test]
    async fn test_new_xl_storage() {
        let tmp_dir = tempdir_in(".").unwrap();
        let mut tmp_path =
            PathBuf::from_path_buf(tmp_dir.path().to_path_buf()).expect("utils.PathBuf");
        tmp_path.push("noexist");
        let tmp_dir_name = String::from(tmp_path.to_str().unwrap());
        tmp_path.pop();
        tmp_path.push("tmpfile");
        let tmp_file_name = String::from(tmp_path.to_str().unwrap());
        let tmp_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .sync()
                .open(&tmp_file_name)
                .await,
            "Unable to create file. {:?}",
            tmp_file_name
        );

        // List of all tests for xlStorage initialization.
        let cases: [(&str, bool, Option<anyhow::Error>); 3] = [
            // Validates input argument cannot be empty.
            ("", true, Some(TypedError::InvalidArgument.into())),
            // Validates if the directory does not exist and
            // gets automatically created.
            #[cfg(windows)]
            (
                &tmp_dir_name,
                false,
                //TODO: unknow error "The filename, directory name, or volume label syntax is incorrect. (os error 123)"
                None,
            ),
            #[cfg(not(windows))]
            (&tmp_dir_name, false, None),
            // Validates if the disk exists as file and returns error
            // not a directory.
            #[cfg(windows)]
            (
                &tmp_file_name,
                false,
                //TODO: unknow error "The filename, directory name, or volume label syntax is incorrect. (os error 123)"
                None,
            ),
            #[cfg(not(windows))]
            (&tmp_file_name, true, Some(StorageError::DiskNotDir.into())),
        ];
        // Validate all test cases.
        for (name, is_err, expected_err) in cases.iter() {
            // Initialize a new xlStorage layer.
            let result = XlStorage::new_local(name).await;
            match result {
                Ok(_) => assert!(!is_err),
                Err(err) => {
                    if *is_err {
                        assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string())
                    }
                }
            }
        }
    }

    // test_xl_storage_make_vol - TestXLStorage validate the logic
    // for creation of new xlStorage volume.
    // Asserts the failures too against the expected failures.
    #[tokio::test]
    async fn test_xl_storage_make_vol() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        // Setup test environment.
        // Create a file.
        #[cfg(windows)]
        let mut vol_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .sync()
                .open(&path_join(&[path.to_str().unwrap(), "vol-as-file"]))
                .await,
            "Unable to create file."
        );
        #[cfg(not(windows))]
        let mut vol_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .mode(0o777)
                .sync()
                .open(&path_join(&[path.to_str().unwrap(), "vol-as-file"]))
                .await,
            "Unable to create file."
        );
        assert_ok!(vol_file.write_all(b"").await, "Unable to write file.");
        // Create a directory.
        assert_ok!(
            mkdir_all(&path_join(&[path.to_str().unwrap(), "existing-vol"]), 0o777).await,
            "Unable to create directory."
        );

        let cases: [(&str, bool, Option<anyhow::Error>); 4] = [
            // TestXLStorage case - 1.
            // A valid case, volume creation is expected to succeed.
            ("success-vol", false, None),
            // TestXLStorage case - 2.
            // Case where a file exists by the name of the volume to be created.
            ("vol-as-file", true, Some(StorageError::VolumeExists.into())),
            // TestXLStorage case - 3.
            (
                "existing-vol",
                true,
                Some(StorageError::VolumeExists.into()),
            ),
            // TestXLStorage case - 5.
            // TestXLStorage case with invalid volume name.
            ("ab", true, Some(TypedError::InvalidArgument.into())),
        ];

        for (vol_name, is_err, expected_err) in cases.iter() {
            let result = xl_storage.make_volume(vol_name).await;
            match result {
                Ok(_) => assert!(!is_err),
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }

        // TestXLStorage for permission denied.
        #[cfg(not(windows))]
        {
            let tmp_dir = tempdir_in(".").unwrap();
            let perm_denied_dir =
                PathBuf::from_path_buf(tmp_dir.path().to_path_buf()).expect("utils.PathBuf");
            assert_ok!(
                set_permissions(perm_denied_dir.as_path(), Permissions::from_mode(0o400)).await,
                "Unable to change permission to temporary directory {:?}",
                perm_denied_dir
            );
            // Initialize xlStorage storage layer for permission denied error.
            let err = XlStorage::new_local(perm_denied_dir.to_str().unwrap()).await;
            match err {
                Ok(_) => assert!(false, "Should unable to initialize xlStorage"),
                Err(err) => assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string()),
            }
            assert_ok!(
                set_permissions(perm_denied_dir.as_path(), Permissions::from_mode(0o755)).await,
                "Unable to change permission to temporary directory {:?}",
                perm_denied_dir
            );
            let xl_storage_new = assert_ok!(
                XlStorage::new_local(perm_denied_dir.to_str().unwrap()).await,
                "Unable to initialize xlStorage"
            );
            // change backend permissions for MakeVol error.
            assert_ok!(
                set_permissions(perm_denied_dir.as_path(), Permissions::from_mode(0o400)).await,
                "Unable to change permission to temporary directory {:?}",
                perm_denied_dir
            );
            let err = assert_err!(xl_storage_new.make_volume("test-vol").await);
            assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string());
        }
    }

    // test_xl_storage_delete_vol - Validates the expected behavior
    // of xlStorage.delete_vol for various cases.
    #[tokio::test]
    async fn test_xl_storage_delete_vol() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        // Setup test environment.
        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create volume"
        );

        // TestXLStorage failure cases.
        let vol = path_join(&[path.to_str().unwrap(), "nonempty-vol"]);
        assert_ok!(mkdir_all(&vol, 0o777).await, "Unable to create directory.");
        #[cfg(windows)]
        let mut vol_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .sync()
                .open(&path_join(&[&vol, "test-file"]))
                .await,
            "Unable to create file."
        );
        #[cfg(not(windows))]
        let mut vol_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .mode(0o777)
                .sync()
                .open(&path_join(&[&vol, "test-file"]))
                .await,
            "Unable to create file."
        );
        assert_ok!(vol_file.write_all(b"").await, "Unable to write file.");

        let cases: [(&str, Option<StorageError>); 4] = [
            // A valida case. Empty vol, should be possible to delete.
            ("success-vol", None),
            // volume is non-existent.
            ("nonexistent-vol", Some(StorageError::VolumeNotFound.into())),
            // It shouldn't be possible to delete an non-empty volume, validating the same.
            ("nonempty-vol", Some(StorageError::VolumeNotEmpty.into())),
            // Invalid volume name.
            ("ab", Some(StorageError::VolumeNotFound.into())),
        ];
        for (vol_name, expected_err) in cases.iter() {
            let result = xl_storage.delete_volume(vol_name, false).await;
            match result {
                Ok(_) => assert_eq!(None, *expected_err),
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }

        // TestXLStorage for permission denied.
        #[cfg(not(windows))]
        {
            let tmp_dir = tempdir_in(".").unwrap();
            let perm_denied_dir =
                PathBuf::from_path_buf(tmp_dir.path().to_path_buf()).expect("utils.PathBuf");

            let mybucket_path = path_join(&[perm_denied_dir.to_str().unwrap(), "mybucket"]);
            assert_ok!(
                mkdir_all(&mybucket_path, 0o400).await,
                "Unable to create temporary directory."
            );
            assert_ok!(
                set_permissions(perm_denied_dir.as_path(), Permissions::from_mode(0o400)).await,
                "Unable to change permission to temporary directory {:?}",
                perm_denied_dir
            );

            // Initialize xlStorage storage layer for permission denied error.
            let err = XlStorage::new_local(perm_denied_dir.to_str().unwrap()).await;
            match err {
                Ok(_) => assert!(false, "Should unable to initialize xlStorage"),
                Err(err) => assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string()),
            }
            assert_ok!(
                set_permissions(perm_denied_dir.as_path(), Permissions::from_mode(0o755)).await,
                "Unable to change permission to temporary directory {:?}",
                perm_denied_dir
            );
            let xl_storage_new = assert_ok!(
                XlStorage::new_local(perm_denied_dir.to_str().unwrap()).await,
                "Unable to initialize xlStorage"
            );
            // change backend permissions for MakeVol error.
            assert_ok!(
                set_permissions(perm_denied_dir.as_path(), Permissions::from_mode(0o400)).await,
                "Unable to change permission to temporary directory {:?}",
                perm_denied_dir
            );
            let err = assert_err!(xl_storage_new.delete_volume("mybucket", false).await);
            assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string());

            remove_perm_denied_file(perm_denied_dir.to_str().unwrap()).await;
        }

        let xl_storages_delete = new_xl_storage_test_setup().await;
        let (xl_storage_delete, disk_path) =
            assert_ok!(xl_storages_delete, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(disk_path.as_std_path());}
        // TestXLStorage for delete on an removed disk.
        // should fail with disk not found.
        let err = assert_err!(xl_storage_delete.delete_volume("Del-Vol", false).await);
        assert_eq!(err.to_string(), StorageError::VolumeNotFound.to_string());
    }

    // test_xl_storage_stat_vol - TestXLStorages validate the volume
    // info returned by xlStorage.StatVol() for various inputs.
    #[tokio::test]
    async fn test_xl_storage_stat_vol() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        // Setup test environment.
        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create volume"
        );

        let cases: [(&str, Option<StorageError>); 3] = [
            ("success-vol", None),
            ("nonexistent-vol", Some(StorageError::VolumeNotFound.into())),
            ("ab", Some(StorageError::VolumeNotFound.into())),
        ];

        for (vol_name, expected_err) in cases.iter() {
            let result = xl_storage.stat_volume(vol_name).await;
            match result {
                Ok(info) => {
                    assert_eq!(None, *expected_err);
                    assert_eq!(&info.name, *vol_name);
                }
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }

        let xl_storages_delete = new_xl_storage_test_setup().await;
        let (xl_storage_delete, disk_path) =
            assert_ok!(xl_storages_delete, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(disk_path.as_std_path());}

        // TestXLStorage for delete on an removed disk.
        // should fail with disk not found.
        let result_delete = xl_storage_delete.stat_volume("Stat vol").await;
        match result_delete {
            Ok(_) => assert!(false, "Expected Error"),
            Err(err) => assert_eq!(err.to_string(), StorageError::VolumeNotFound.to_string()),
        }
    }

    // test_xl_storage_list_vols - Validates the result and the error output
    // for xlStorage volume listing functionality xlStorage.ListVols().
    #[tokio::test]
    async fn test_xl_storage_list_vols() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");

        // TestXLStorage empty list vols.
        let vol_infos = assert_ok!(xl_storage.list_volumes().await);
        assert_eq!(vol_infos.len(), 1);

        // TestXLStorage non-empty list vols.
        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create volume"
        );
        let vol_infos = assert_ok!(xl_storage.list_volumes().await);
        assert_eq!(vol_infos.len(), 2);
        let mut vol_found = false;
        for info in vol_infos {
            if info.name == "success-vol" {
                vol_found = true;
                break;
            }
        }
        assert!(vol_found, "expected: success-vol to be created");

        // removing the path and simulating disk failure
        assert_ok!(std::fs::remove_dir_all(path.as_std_path()));
        // should fail with errDiskNotFound.
        let result_delete = xl_storage.list_volumes().await;
        match result_delete {
            Ok(_) => assert!(false, "Expected Error"),
            Err(err) => assert_eq!(err.to_string(), StorageError::DiskNotFound.to_string()),
        }
    }

    // test_xl_storage_list_dir - TestXLStorages validate the directory
    // listing functionality provided by xlStorage.ListDir.
    #[tokio::test]
    async fn test_xl_storage_list_dir() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        let xl_storages_delete = new_xl_storage_test_setup().await;
        let (xl_storage_delete, disk_path) =
            assert_ok!(xl_storages_delete, "Unable to create xlStorage test setup.");
        // removing the disk, used to recreate disk not found error.
        assert_ok!(remove_all(disk_path.as_path()).await);

        // Setup test environment.
        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create a volume."
        );
        assert_ok!(
            xl_storage
                .append_file("success-vol", "abc/def/ghi/success-file", b"Hello, world")
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .append_file("success-vol", "abc/xyz/ghi/success-file", b"Hello, world")
                .await,
            "Unable to create file"
        );

        let cases: [(&str, &str, Vec<&str>, Option<StorageError>); 6] = [
            // valid case with existing volume and file to delete.
            ("success-vol", "abc", vec!["def/", "xyz/"], None),
            ("success-vol", "abc/def", vec!["ghi/"], None),
            ("success-vol", "abc/def/ghi", vec!["success-file"], None),
            (
                "success-vol",
                "abcdef",
                Vec::new(),
                Some(StorageError::FileNotFound.into()),
            ),
            (
                "ab",
                "success-file",
                Vec::new(),
                Some(StorageError::VolumeNotFound.into()),
            ),
            (
                "non-existent-vol",
                "success-file",
                Vec::new(),
                Some(StorageError::VolumeNotFound.into()),
            ),
        ];

        for (src_vol, src_path, expected_list_dir, expected_err) in cases.iter() {
            let result = xl_storage.list_dir(src_vol, src_path, 0).await;
            match result {
                Ok(dir_list) => {
                    let dir_list_join = dir_list.join(",");
                    assert_eq!(None, *expected_err);
                    for expected in expected_list_dir {
                        assert!(dir_list_join.contains(expected));
                    }
                }
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }

        // TestXLStorage for permission denied.
        #[cfg(not(windows))]
        {
            let perm_denied_dir = create_perm_denied_file().await;

            // Initialize xlStorage storage layer for permission denied error.
            let err = XlStorage::new_local(&perm_denied_dir).await;
            match err {
                Ok(_) => assert!(false, "Should unable to initialize xlStorage"),
                Err(err) => assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string()),
            }
            assert_ok!(
                set_permissions(&perm_denied_dir, Permissions::from_mode(0o755)).await,
                "Unable to change permission to temporary directory {:?}",
                &perm_denied_dir
            );
            let xl_storage_new = assert_ok!(
                XlStorage::new_local(&perm_denied_dir).await,
                "Unable to initialize xlStorage"
            );

            let err = assert_err!(xl_storage_new.delete("mybucket", "myobject", false).await);
            assert_eq!(
                err.to_string(),
                StorageError::VolumeAccessDenied.to_string()
            );

            remove_perm_denied_file(&perm_denied_dir).await;
        }

        // TestXLStorage for delete on an removed disk.
        // should fail with disk not found.
        let err = assert_err!(xl_storage_delete.delete("del-vol", "my-file", false).await);
        assert_eq!(err.to_string(), StorageError::DiskNotFound.to_string());
    }

    // test_xl_storage_delete_file - Series of test cases construct valid
    // and invalid input data and validates the result and the error response.
    #[cfg(not(windows))]
    #[tokio::test]
    async fn test_xl_storage_delete_file() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        let xl_storages_delete = new_xl_storage_test_setup().await;
        let (xl_storage_delete, disk_path) =
            assert_ok!(xl_storages_delete, "Unable to create xlStorage test setup.");
        // removing the disk, used to recreate disk not found error.
        assert_ok!(remove_all(disk_path.as_path()).await);

        // Setup test environment.
        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create a volume"
        );
        assert_ok!(
            xl_storage
                .append_file("success-vol", "success-file", b"Hello, world")
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage.make_volume("no-permissions").await,
            "Unable to create a volume"
        );
        assert_ok!(
            xl_storage
                .append_file("no-permissions", "dir/file", b"Hello, world")
                .await,
            "Unable to create file"
        );
        // Parent directory must have write permissions, this is read + execute.
        assert_ok!(
            set_permissions(
                &path_join(&[path.to_str().unwrap(), "no-permissions"]),
                Permissions::from_mode(0o555)
            )
            .await,
            "Unable to change permission to no-permissions"
        );

        let cases: [(&str, &str, Option<StorageError>); 6] = [
            // valid case with existing volume and file to delete.
            ("success-vol", "success-file", None),
            // The file was deleted in the last  case, so Delete should not fail.
            ("success-vol", "success-file", None),
            // TestXLStorage case with segment of the volume name > 255.
            (
                "my",
                "success-file",
                Some(StorageError::VolumeNotFound.into())
            ),
            // TestXLStorage case with non-existent volume.
            (
                "non-existent-vol",
                "success-file",
                Some(StorageError::VolumeNotFound.into())
            ),
            // TestXLStorage case with src path segment > 255.
            (
                "success-vol",
                "my-obj-del-0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001", 
                Some(StorageError::FileNameTooLong.into())
            ),
            // TestXLStorage case with undeletable parent directory.
            // File can delete, dir cannot delete because no-permissions doesn't have write perms.
            ("no-permissions", "dir/file", Some(StorageError::VolumeAccessDenied.into())),
        ];

        for (src_vol, src_path, expected_err) in cases.iter() {
            let result = xl_storage.delete(src_vol, src_path, false).await;
            match result {
                Ok(_) => assert_eq!(None, *expected_err),
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }

        // TestXLStorage for permission denied.
        let perm_denied_dir = create_perm_denied_file().await;

        // Initialize xlStorage storage layer for permission denied error.
        let err = XlStorage::new_local(&perm_denied_dir).await;
        match err {
            Ok(_) => assert!(false, "Should unable to initialize xlStorage"),
            Err(err) => assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string()),
        }
        assert_ok!(
            set_permissions(&perm_denied_dir, Permissions::from_mode(0o755)).await,
            "Unable to change permission to temporary directory {:?}",
            &perm_denied_dir
        );
        let xl_storage_new = assert_ok!(
            XlStorage::new_local(&perm_denied_dir).await,
            "Unable to initialize xlStorage"
        );

        let err = assert_err!(xl_storage_new.delete("mybucket", "myobject", false).await);
        assert_eq!(
            err.to_string(),
            StorageError::VolumeAccessDenied.to_string()
        );

        remove_perm_denied_file(&perm_denied_dir).await;

        // TestXLStorage for delete on an removed disk.
        // should fail with disk not found.
        let err = assert_err!(xl_storage_delete.delete("del-vol", "my-file", false).await);
        assert_eq!(err.to_string(), StorageError::DiskNotFound.to_string());

        assert_ok!(
            set_permissions(
                &path_join(&[path.to_str().unwrap(), "no-permissions"]),
                Permissions::from_mode(0o755)
            )
            .await,
            "Unable to change permission to no-permissions"
        );
    }

    // test_xl_storage_read_file - TestXLStorages xlStorage.ReadFile with
    // wide range of cases and asserts the result and error response.
    #[tokio::test]
    async fn test_xl_storage_read_file() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        // Setup test environment.
        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create a volume"
        );
        assert_ok!(
            mkdir_all(
                path_join(&[path.to_str().unwrap(), "success-vol", "object-as-dir"]),
                0o777
            )
            .await,
            "Unable to create directory. {:?}",
            path
        );

        let cases: [(
            &str,
            &str,
            u64,
            usize,
            &[u8],
            Option<StorageError>,
            Option<std::io::ErrorKind>,
        ); 13] = [
            // Successful read at offset 0 and proper buffer size. - 1
            ("success-vol", "myobject", 0, 5, b"hello", None, None),
            // Success read at hierarchy. - 2
            (
                "success-vol",
                "path/to/my/object",
                0,
                5,
                b"hello",
                None,
                None,
            ),
            // Object is a directory. - 3
            (
                "success-vol",
                "object-as-dir",
                0,
                5,
                b"",
                Some(StorageError::IsNotRegular.into()),
                None,
            ),
            // One path segment length is > 255 chars long. - 4
            (
                "success-vol",
                "path/to/my/object0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
                0,
                5,
                b"",
                Some(StorageError::FileNameTooLong.into()),
                None
            ),
            // Path length is > 1024 chars long. - 5
            (
                "success-vol",
                "level0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001/level0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002/level0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003/object000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
                0,
                5,
                b"",
                Some(StorageError::FileNameTooLong.into()),
                None
            ),
            // Buffer size greater than object size. - 6
            (
                "success-vol",
                "myobject",
                0,
                16,
                b"hello, world",
                None,
                Some(ErrorKind::UnexpectedEof.into()),
            ),
            // Reading from an offset success. - 7
            ("success-vol", "myobject", 7, 5, b"world", None, None),
            // Reading from an object but buffer size greater. - 8
            (
                "success-vol",
                "myobject",
                7,
                8,
                b"world",
                None,
                Some(ErrorKind::UnexpectedEof.into()),
            ),
            // Seeking ahead returns io.EOF. - 9
            (
                "success-vol",
                "myobject",
                14,
                1,
                b"",
                None,
                Some(ErrorKind::UnexpectedEof.into()),
            ),
            // Empty volume name. - 10
            (
                "",
                "myobject",
                14,
                1,
                b"",
                Some(StorageError::VolumeNotFound.into()),
                None,
            ),
            // Empty filename name. - 11
            (
                "success-vol",
                "",
                14,
                1,
                b"",
                Some(StorageError::IsNotRegular.into()),
                None,
            ),
            // Non existent volume name - 12
            (
                "abcd",
                "",
                14,
                1,
                b"",
                Some(StorageError::VolumeNotFound.into()),
                None,
            ),
            // Non existent filename - 13
            (
                "success-vol",
                "abcd",
                14,
                1,
                b"",
                Some(StorageError::FileNotFound.into()),
                None,
            ),
        ];
        // Create all files needed during testing.
        // SHA256Sum([]byte("hello, world")
        for (
            i,
            (volume, file_name, offset, buf_size, expected_buf, expected_err, expected_io_err),
        ) in cases.iter().enumerate()
        {
            if i > 4 {
                break;
            }
            let result = xl_storage
                .append_file(volume, file_name, b"hello, world")
                .await;
            match result {
                Ok(_) => assert_eq!(None, *expected_err),
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }

        let mut buf = [0u8; 5];
        let v = BitrotVerifier {
            algorithm: BitrotAlgorithm::HighwayHash256,
            hash: [
                187, 165, 33, 249, 196, 92, 26, 88, 44, 104, 111, 20, 38, 161, 81, 148, 225, 51,
                180, 148, 62, 207, 207, 43, 126, 71, 136, 163, 145, 52, 232, 102,
            ],
            // TODO: not support HASH256. use HighwayHash256 instead
            // hash: [
            //     9, 202, 126, 78, 170, 110, 138, 233, 199, 210, 97, 22, 113, 41, 24, 72, 131, 100,
            //     77, 7, 223, 186, 124, 191, 188, 76, 138, 46, 8, 54, 13, 91,
            // ],
        };
        let err = assert_err!(
            xl_storage
                .read_file("success-vol", "myobject", 100, &mut buf, Some(v))
                .await,
            "expected: error, got: <nil>"
        );

        let mut l = 0;
        while l < 2 {
            // 1st loop tests with dma=write, 2nd loop tests with dma=read-write.
            if l == 1 {
                globals::GLOBALS.storage_class.guard().dma =
                    String::from(crate::config::storageclass::DMA_READ_WRITE);
            }
            // Following block validates all ReadFile test cases.
            for (
                i,
                (volume, file_name, offset, buf_size, expected_buf, expected_err, expected_io_err),
            ) in cases.iter().enumerate()
            {
                // Common read buffer.
                let mut buf = vec![0u8; *buf_size];
                let v = BitrotVerifier {
                    algorithm: BitrotAlgorithm::HighwayHash256,
                    hash: [
                        187, 165, 33, 249, 196, 92, 26, 88, 44, 104, 111, 20, 38, 161, 81, 148,
                        225, 51, 180, 148, 62, 207, 207, 43, 126, 71, 136, 163, 145, 52, 232, 102,
                    ],
                    // TODO: not support HASH256. use HighwayHash256 instead
                    // hash: [
                    //     9, 202, 126, 78, 170, 110, 138, 233, 199, 210, 97, 22, 113, 41, 24, 72, 131, 100,
                    //     77, 7, 223, 186, 124, 191, 188, 76, 138, 46, 8, 54, 13, 91,
                    // ],
                };
                let result = xl_storage
                    .read_file(*volume, *file_name, *offset, &mut buf, Some(v))
                    .await;
                match result {
                    Ok(n) => {
                        assert_eq!(None, *expected_err);
                        assert_eq!(None, *expected_io_err);
                        assert_eq!(buf, *expected_buf);
                        assert_eq!(n as usize, *buf_size);
                    }
                    Err(err) => {
                        if *expected_err != None {
                            assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string());
                        } else if *expected_io_err != None {
                            assert_eq!(err.to_string(), "unexpected end of file");
                            assert!(buf.starts_with(expected_buf));
                        } else {
                            assert!(false, "expected err");
                        }
                    }
                }
            }
            l += 1;
        }

        // Reset the flag.
        globals::GLOBALS.storage_class.guard().dma =
            String::from(crate::config::storageclass::DMA_WRITE);

        // TestXLStorage for permission denied.
        #[cfg(not(windows))]
        {
            let perm_denied_dir = create_perm_denied_file().await;

            // Initialize xlStorage storage layer for permission denied error.
            let err = XlStorage::new_local(&perm_denied_dir).await;
            match err {
                Ok(_) => assert!(false, "Should unable to initialize xlStorage"),
                Err(err) => assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string()),
            }
            assert_ok!(
                set_permissions(&perm_denied_dir, Permissions::from_mode(0o755)).await,
                "Unable to change permission to temporary directory {:?}",
                &perm_denied_dir
            );

            let xl_storage_perm = assert_ok!(
                XlStorage::new_local(&perm_denied_dir).await,
                "Unable to initialize xlStorage"
            );

            // Common read buffer.
            let mut buf = [0u8; 10];
            let v = BitrotVerifier {
                algorithm: BitrotAlgorithm::HighwayHash256,
                hash: [
                    187, 165, 33, 249, 196, 92, 26, 88, 44, 104, 111, 20, 38, 161, 81, 148, 225,
                    51, 180, 148, 62, 207, 207, 43, 126, 71, 136, 163, 145, 52, 232, 102,
                ],
                // TODO: not support HASH256. use HighwayHash256 instead
                // hash: [
                //     9, 202, 126, 78, 170, 110, 138, 233, 199, 210, 97, 22, 113, 41, 24, 72, 131, 100,
                //     77, 7, 223, 186, 124, 191, 188, 76, 138, 46, 8, 54, 13, 91,
                // ],
            };
            let err = assert_err!(
                xl_storage_perm
                    .read_file("mybucket", "myobject", 0, &mut buf, Some(v))
                    .await
            );
            assert_eq!(err.to_string(), StorageError::FileAccessDenied.to_string());

            remove_perm_denied_file(&perm_denied_dir).await;
        }
    }

    const XL_STORAGE_READ_FILE_WITH_VERIFY_TESTS: [(
        &str,                 // file
        u64,                  // offset
        usize,                // length
        BitrotAlgorithm,      // algorithm
        Option<StorageError>, // expError
    ); 16] = [
        ("myobject", 0, 100, BitrotAlgorithm::SHA256, None), // 0
        ("myobject", 25, 74, BitrotAlgorithm::SHA256, None), // 1
        ("myobject", 29, 70, BitrotAlgorithm::SHA256, None), // 2
        ("myobject", 100, 0, BitrotAlgorithm::SHA256, None), // 3
        (
            "myobject",
            1,
            120,
            BitrotAlgorithm::SHA256,
            Some(StorageError::FileCorrupt),
        ), // 4
        ("myobject", 3, 1100, BitrotAlgorithm::SHA256, None), // 5
        (
            "myobject",
            2,
            100,
            BitrotAlgorithm::SHA256,
            Some(StorageError::FileCorrupt),
        ), // 6
        ("myobject", 1000, 1001, BitrotAlgorithm::SHA256, None), // 7
        (
            "myobject",
            0,
            100,
            BitrotAlgorithm::BLAKE2b512,
            Some(StorageError::FileCorrupt),
        ), // 8
        ("myobject", 25, 74, BitrotAlgorithm::BLAKE2b512, None), // 9
        (
            "myobject",
            29,
            70,
            BitrotAlgorithm::BLAKE2b512,
            Some(StorageError::FileCorrupt),
        ), // 10
        ("myobject", 100, 0, BitrotAlgorithm::BLAKE2b512, None), // 11
        ("myobject", 1, 120, BitrotAlgorithm::BLAKE2b512, None), // 12
        ("myobject", 3, 1100, BitrotAlgorithm::BLAKE2b512, None), // 13
        ("myobject", 2, 100, BitrotAlgorithm::BLAKE2b512, None), // 14
        ("myobject", 1000, 1001, BitrotAlgorithm::BLAKE2b512, None), // 15
    ];

    // TestXLStorageReadFile with bitrot verification - tests the xlStorage level
    // ReadFile API with a BitrotVerifier. Only tests hashing related
    // functionality. Other functionality is tested with
    // TestXLStorageReadFile.
    #[tokio::test]
    async fn test_xl_storage_read_file_with_verify() {
        let volume = "test-vol";
        let object = "myobject";
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        assert_ok!(
            xl_storage.make_volume(volume).await,
            "Unable to create a volume"
        );

        let mut aligned_buf = fs::AlignedBlock::new(8192);
        utils::rng_seed_now().fill(&mut *aligned_buf);

        assert_ok!(
            xl_storage
                .append_file(volume, object, aligned_buf.as_ref())
                .await,
            "Unable to create object"
        );
        let data = aligned_buf.to_vec();

        for (file, offset, length, algorithm, exp_err) in
            XL_STORAGE_READ_FILE_WITH_VERIFY_TESTS.iter()
        {
            let mut h = algorithm.hasher();
            h.append(&data);
            if *exp_err != None {
                h.append(&[0u8; 1]);
            }
            let mut buffer = vec![0u8; *length];
            let hash = h.finish();
            let hash = hash.try_into().unwrap();
            let v = BitrotVerifier {
                algorithm: *algorithm,
                hash,
            };
            let result = xl_storage
                .read_file(volume, file, *offset, &mut buffer, Some(v))
                .await;
            match result {
                Ok(n) => {
                    assert_eq!(None, *exp_err);
                    assert_eq!(n, *length as u64);
                    assert_eq!(buffer, data[*offset as usize..(*offset as usize + *length)]);
                }
                Err(err) => assert_eq!(err.to_string(), exp_err.as_ref().unwrap().to_string()),
            }
        }
    }

    // test_xl_storage_format_file_change - to test if changing the diskID
    // makes the calls fail.
    #[tokio::test]
    async fn test_xl_storage_format_file_change() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        assert_ok!(
            xl_storage.make_volume("testvolume").await,
            "Unable to create a volume"
        );

        // Change the format.json such that "this" is changed to "randomid".
        #[cfg(windows)]
        let mut vol_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .sync()
                .open(&path_join(&[
                    path.to_str().unwrap(),
                    SYSTEM_META_BUCKET,
                    FORMAT_CONFIG_FILE
                ]))
                .await,
            "Unable to create file."
        );
        #[cfg(not(windows))]
        let mut vol_file = assert_ok!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .write(true)
                .mode(0o644)
                .sync()
                .open(&path_join(&[
                    path.to_str().unwrap(),
                    SYSTEM_META_BUCKET,
                    FORMAT_CONFIG_FILE
                ]))
                .await,
            "Unable to create file."
        );
        assert_ok!(vol_file.write_all(r#"{"version":"1","format":"xl","id":"592a41c2-b7cc-4130-b883-c4b5cb15965b","xl":{"version":"3","this":"randomid","sets":[["e07285a6-8c73-4962-89c6-047fb939f803","33b8d431-482d-4376-b63c-626d229f0a29","cff6513a-4439-4dc1-bcaa-56c9e880c352","randomid","9c9f21d5-1f15-4737-bce6-835faa0d9626","0a59b346-1424-4fc2-9fa2-a2e80541d0c1","7924a3dc-b69a-4971-9a2e-014966d6aebb","4d2b8dd9-4e48-444b-bdca-c89194b26042"]],"distributionAlgo":"CRCMOD"}}"#.as_bytes()).await, "Unable to write file.");

        let err = assert_err!(
            xl_storage.make_volume("testvolume").await,
            "Unable to create a volume"
        );
        assert_eq!(err.to_string(), StorageError::VolumeExists.to_string());
    }

    // TestXLStorage xlStorage.AppendFile()
    #[tokio::test]
    async fn test_xl_storage_append_file() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        // Setup test environment.
        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create a volume"
        );

        // Create directory to make errIsNotRegular
        assert_ok!(
            mkdir_all(
                path_join(&[path.to_str().unwrap(), "success-vol", "object-as-dir"]),
                0o777
            )
            .await,
            "Unable to create directory. {:?}",
            path
        );

        let cases: [(&str, Option<StorageError>); 8] = [
            ("myobject", None),
            ("path/to/my/object", None),
            // TestXLStorage to append to previously created file.
            ("myobject", None),
            // TestXLStorage to use same path of previously created file.
            ("path/to/my/testobject", None),
            // TestXLStorage to use object is a directory now.
            ("object-as-dir", Some(StorageError::IsNotRegular.into())),
            // path segment uses previously uploaded object.
            (
                "myobject/testobject",
                Some(StorageError::FileAccessDenied.into()),
            ),
            // One path segment length is > 255 chars long.
		    (
                "path/to/my/object0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001", 
                Some(StorageError::FileNameTooLong.into()),
            ),
		    // path length is > 1024 chars long.
		    (
                "level0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001/level0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002/level0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003/object000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001", 
                Some(StorageError::FileNameTooLong.into()),
            ),
        ];
        for (file_name, expected_err) in cases.iter() {
            let result = xl_storage
                .append_file("success-vol", file_name, b"hello, world")
                .await;
            match result {
                Ok(_) => assert_eq!(None, *expected_err),
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }

        // TestXLStorage for permission denied.
        #[cfg(not(windows))]
        {
            let perm_denied_dir = create_perm_denied_file().await;

            // Initialize xlStorage storage layer for permission denied error.
            let err = XlStorage::new_local(&perm_denied_dir).await;
            match err {
                Ok(_) => assert!(false, "Should unable to initialize xlStorage"),
                Err(err) => assert_eq!(err.to_string(), StorageError::DiskAccessDenied.to_string()),
            }
            assert_ok!(
                set_permissions(&perm_denied_dir, Permissions::from_mode(0o755)).await,
                "Unable to change permission to temporary directory {:?}",
                &perm_denied_dir
            );

            let xl_storage_perm = assert_ok!(
                XlStorage::new_local(&perm_denied_dir).await,
                "Unable to initialize xlStorage"
            );

            let err = assert_err!(
                xl_storage_perm
                    .append_file("mybucket", "myobject", b"hello, world")
                    .await
            );
            assert_eq!(
                err.to_string(),
                StorageError::VolumeAccessDenied.to_string()
            );

            remove_perm_denied_file(&perm_denied_dir).await;
        }

        // TestXLStorage case with invalid volume name.
        // A valid volume name should be atleast of size 3.
        let err = assert_err!(xl_storage.append_file("bh", "yes", b"hello, world").await);
        assert_eq!(err.to_string(), StorageError::VolumeNotFound.to_string());
    }

    // TestXLStorage xlStorage.RenameFile()
    #[tokio::test]
    async fn test_xl_storage_rename_file() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        // Setup test environment.
        assert_ok!(
            xl_storage.make_volume("src-vol").await,
            "Unable to create a volume"
        );
        assert_ok!(
            xl_storage.make_volume("dest-vol").await,
            "Unable to create a volume"
        );
        assert_ok!(
            xl_storage
                .append_file("src-vol", "file1", b"Hello, world")
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .append_file("src-vol", "file2", b"Hello, world")
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .append_file("src-vol", "file3", b"Hello, world")
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .append_file("src-vol", "file4", b"Hello, world")
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .append_file("src-vol", "file5", b"Hello, world")
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .append_file("src-vol", "path/to/file1", b"Hello, world")
                .await,
            "Unable to create file"
        );

        let cases: [(&str, &str, &str, &str, Option<StorageError>); 17] = [
            // TestXLStorage case - 1.
            ("src-vol", "dest-vol", "file1", "file-one", None),
            // TestXLStorage case - 2.
            ("src-vol", "dest-vol", "path/", "new-path/", None),
            // TestXLStorage case - 3.
            // TestXLStorage to overwrite destination file.
            ("src-vol", "dest-vol", "file2", "file-one", None),
            // TestXLStorage case - 4.
            // TestXLStorage case with io error count set to 1.
            // expected not to fail.
            ("src-vol", "dest-vol", "file3", "file-two", None),
            // TestXLStorage case - 5.
            // TestXLStorage case with io error count set to maximum allowed count.
            // expected not to fail.
            ("src-vol", "dest-vol", "file4", "file-three", None),
            // TestXLStorage case - 6.
            // TestXLStorage case with non-existent source file.
            (
                "src-vol",
                "dest-vol",
                "non-existent-file",
                "file-three",
                Some(StorageError::FileNotFound.into()),
            ),
            // TestXLStorage case - 7.
            // TestXLStorage to check failure of source and destination are not same type.
            (
                "src-vol",
                "dest-vol",
                "path/",
                "file-one",
                Some(StorageError::FileAccessDenied.into()),
            ),
            // TestXLStorage case - 8.
            // TestXLStorage to check failure of destination directory exists.
            (
                "src-vol",
                "dest-vol",
                "path/",
                "new-path/",
                Some(StorageError::FileAccessDenied.into()),
            ),
            // TestXLStorage case - 9.
            // TestXLStorage case with source being a file and destination being a directory.
            // Either both have to be files or directories.
            // Expecting to fail with `errFileAccessDenied`.
            (
                "src-vol",
                "dest-vol",
                "file4",
                "new-path/",
                Some(StorageError::FileAccessDenied.into()),
            ),
            // TestXLStorage case - 10.
            // TestXLStorage case with non-existent source volume.
            // Expecting to fail with `errVolumeNotFound`.
            (
                "src-vol-non-existent",
                "dest-vol",
                "file4",
                "new-path/",
                Some(StorageError::VolumeNotFound.into()),
            ),
            // TestXLStorage case - 11.
            // TestXLStorage case with non-existent destination volume.
            // Expecting to fail with `errVolumeNotFound`.
            (
                "src-vol",
                "dest-vol-non-existent",
                "file4",
                "new-path/",
                Some(StorageError::VolumeNotFound.into()),
            ),
            // TestXLStorage case - 12.
            // TestXLStorage case with invalid src volume name. Length should be atleast 3.
            // Expecting to fail with `errInvalidArgument`.
            (
                "ab",
                "dest-vol-non-existent",
                "file4",
                "new-path/",
                Some(StorageError::VolumeNotFound.into()),
            ),
            // TestXLStorage case - 13.
            // TestXLStorage case with invalid destination volume name. Length should be atleast 3.
            // Expecting to fail with `errInvalidArgument`.
            (
                "abcd",
                "ef",
                "file4",
                "new-path/",
                Some(StorageError::VolumeNotFound.into()),
            ),
            // TestXLStorage case - 14.
            // TestXLStorage case with invalid destination volume name. Length should be atleast 3.
            // Expecting to fail with `errInvalidArgument`.
            (
                "abcd",
                "ef",
                "file4",
                "new-path/",
                Some(StorageError::VolumeNotFound.into()),
            ),
            // TestXLStorage case - 15.
            // TestXLStorage case with the parent of the destination being a file.
            // expected to fail with `errFileAccessDenied`.
            (
                "src-vol",
                "dest-vol",
                "file5",
                "file-one/parent-is-file",
                Some(StorageError::FileAccessDenied.into()),
            ),
            // TestXLStorage case - 16.
            // TestXLStorage case with segment of source file name more than 255.
            // expected not to fail.
            (
                "src-vol",
                "dest-vol",
                "path/to/my/object0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
                "file-six",
                Some(StorageError::FileNameTooLong.into()),
            ),
            // TestXLStorage case - 17.
            // TestXLStorage case with segment of destination file name more than 255.
            // expected not to fail.
            (
                "src-vol",
                "dest-vol",
                "file6",
                "path/to/my/object0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
                Some(StorageError::FileNameTooLong.into()),
            ),
        ];

        for (src_vol, dest_vol, src_path, dest_path, expected_err) in cases.iter() {
            let result = xl_storage
                .rename_file(src_vol, src_path, dest_vol, dest_path)
                .await;
            match result {
                Ok(_) => assert_eq!(None, *expected_err),
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }
    }

    // TestXLStorage xlStorage.CheckFile()
    #[tokio::test]
    async fn test_xl_storage_check_file() {
        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        assert_ok!(
            xl_storage.make_volume("success-vol").await,
            "Unable to create a volume"
        );
        assert_ok!(
            xl_storage
                .append_file(
                    "success-vol",
                    &path_join(&["success-file", XL_STORAGE_FORMAT_FILE]),
                    b"Hello, world"
                )
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .append_file(
                    "success-vol",
                    &path_join(&["path/to/success-file", XL_STORAGE_FORMAT_FILE]),
                    b"Hello, world"
                )
                .await,
            "Unable to create file"
        );
        assert_ok!(
            xl_storage
                .make_volume(&path_join(&[
                    "success-vol/path/to/",
                    XL_STORAGE_FORMAT_FILE
                ]))
                .await,
            "Unable to create a volume"
        );

        let cases: [(&str, &str, Option<StorageError>); 7] = [
            // TestXLStorage case - 1.
            // TestXLStorage case with valid inputs, expected to pass.
            ("success-vol", "success-file", None),
            // TestXLStorage case - 2.
            // TestXLStorage case with valid inputs, expected to pass.
            ("success-vol", "path/to/success-file", None),
            // TestXLStorage case - 3.
            // TestXLStorage case with non-existent file.
            (
                "success-vol",
                "nonexistent-file",
                Some(StorageError::PathNotFound.into()),
            ),
            // TestXLStorage case - 4.
            // TestXLStorage case with non-existent file path.
            (
                "success-vol",
                "path/2/success-file",
                Some(StorageError::PathNotFound.into()),
            ),
            // TestXLStorage case - 5.
            // TestXLStorage case with path being a directory.
            (
                "success-vol",
                "path",
                Some(StorageError::PathNotFound.into()),
            ),
            // TestXLStorage case - 6.
            // TestXLStorage case with non existent volume.
            (
                "non-existent-vol",
                "success-file",
                Some(StorageError::PathNotFound.into()),
            ),
            // TestXLStorage case - 7.
            // TestXLStorage case with file with directory.
            (
                "success-vol",
                "path/to",
                Some(StorageError::FileNotFound.into()),
            ),
        ];

        for (src_vol, src_path, expected_err) in cases.iter() {
            let result = xl_storage.check_file(src_vol, src_path).await;
            match result {
                Ok(_) => assert_eq!(None, *expected_err),
                Err(err) => assert_eq!(err.to_string(), expected_err.as_ref().unwrap().to_string()),
            }
        }
    }

    // Test xlStorage.VerifyFile()
    #[tokio::test]
    async fn test_xl_storage_verify_file() {
        // We test 4 cases:
        // 1) Whole-file bitrot check on proper file
        // 2) Whole-file bitrot check on corrupted file
        // 3) Streaming bitrot check on proper file
        // 4) Streaming bitrot check on corrupted file

        let xl_storages = new_xl_storage_test_setup().await;
        let (xl_storage, path) = assert_ok!(xl_storages, "Unable to create xlStorage test setup.");
        defer! {std::fs::remove_dir_all(path.as_std_path());}

        let vol_name = "testvol";
        let file_name = "testfile";
        assert_ok!(
            xl_storage.make_volume(vol_name).await,
            "Unable to create a volume"
        );

        // 1) Whole-file bitrot check on proper file
        let size: u64 = 4 * 1024 * 1024 + 100 * 1024; // 4.1 MB
        let mut data = fs::AlignedBlock::new(size as usize);
        utils::rng_seed_now().fill(&mut *data);
        assert_ok!(
            xl_storage
                .write_all(vol_name, file_name, data.as_ref())
                .await
        );

        let algo = BitrotAlgorithm::HighwayHash256;
        let mut h = algo.hasher();
        h.append(&data);
        let hash_bytes = h.finish();

        // TODO fail
        // let file = assert_ok!(
        //     fs::OpenOptions::new()
        //         .read(true)
        //         .open(&path_join(&[path.to_str().unwrap(), vol_name, file_name]))
        //         .await
        // );
        // let file_size = assert_ok!(file.metadata().await).len();
        // assert_ok!(crate::bitrot::bitrot_verify(file, file_size, size, algo, hash_bytes, 0).await);

        // 2) Whole-file bitrot check on corrupted file
        assert_ok!(
            xl_storage.append_file(vol_name, file_name, b"a").await,
            "Unable to append file"
        );

        // Check if VerifyFile reports the incorrect file length (the correct length is `size+1`)

        let file = assert_ok!(
            fs::OpenOptions::new()
                .read(true)
                .open(&path_join(&[path.to_str().unwrap(), vol_name, file_name]))
                .await
        );
        let file_size = assert_ok!(file.metadata().await).len();
        assert_err!(
            crate::bitrot::bitrot_verify(file, file_size, size, algo, hash_bytes, 0).await,
            "expected to fail bitrot check"
        );

        // Check if bitrot fails
        let file = assert_ok!(
            fs::OpenOptions::new()
                .read(true)
                .open(&path_join(&[path.to_str().unwrap(), vol_name, file_name]))
                .await
        );
        let file_size = assert_ok!(file.metadata().await).len();
        assert_err!(
            crate::bitrot::bitrot_verify(file, file_size, (size + 1), algo, hash_bytes, 0).await,
            "expected to fail bitrot check"
        );

        assert_ok!(xl_storage.delete(vol_name, file_name, false).await);

        // 3) Streaming bitrot check on proper file
        let algo = BitrotAlgorithm::HighwayHash256S;
        let shard_size: u64 = 1024 * 1024;
        let mut shard = fs::AlignedBlock::new(shard_size as usize);

        let bitrot_sums_size =
            crate::utils::ceil_frac(size, shard_size) * (std::mem::size_of::<[u64; 4]>() as u64);
        let total_size = Some(bitrot_sums_size + size);
        let writer = assert_ok!(
            xl_storage
                .create_file_writer(vol_name, file_name, total_size)
                .await
        );

        let mut w = crate::bitrot::HighwayBitrotWriter::new(writer);
        for byte in data.bytes() {
            match byte {
                Ok(n) => {
                    w.write(&[n]);
                }
                Err(err) => {
                    assert!(false, "Error: {}", err);
                }
            }
        }
        w.flush();

        // TODO fail
        // let file = assert_ok!(
        //     fs::OpenOptions::new()
        //         .read(true)
        //         .open(&path_join(&[path.to_str().unwrap(), vol_name, file_name]))
        //         .await
        // );
        // let file_size = assert_ok!(file.metadata().await).len();
        // assert_ok!(
        //     crate::bitrot::bitrot_verify(file, file_size, size, algo, b"", shard_size).await,
        //     "expected to fail bitrot check"
        // );

        // 4) Streaming bitrot check on corrupted file
        let mut f = assert_ok!(
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path_join(&[path.to_str().unwrap(), vol_name, file_name]))
                .await
        );

        // Replace first 256 with 'a'.
        let replace_chars = [b'a'; 256];
        assert_ok!(f.seek(SeekFrom::Start(0)).await);
        assert_ok!(f.write_all(&replace_chars).await);

        let file = assert_ok!(
            fs::OpenOptions::new()
                .read(true)
                .open(&path_join(&[path.to_str().unwrap(), vol_name, file_name]))
                .await
        );
        let file_size = assert_ok!(file.metadata().await).len();
        assert_err!(
            crate::bitrot::bitrot_verify(file, file_size, size, algo, b"", shard_size).await,
            "expected to fail bitrot check"
        );

        let file = assert_ok!(
            fs::OpenOptions::new()
                .read(true)
                .open(&path_join(&[path.to_str().unwrap(), vol_name, file_name]))
                .await
        );
        let file_size = assert_ok!(file.metadata().await).len();
        assert_err!(
            crate::bitrot::bitrot_verify(file, file_size, (size + 1), algo, b"", shard_size).await,
            "expected to fail bitrot check"
        );
    }
}
