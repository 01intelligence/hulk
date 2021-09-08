mod datatypes;
mod heal;

pub use datatypes::*;
pub use heal::*;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::xl_storage::XlStorage;
use crate::{bitrot, utils};

pub enum StorageApi {
    XlStorage(XlStorage),
}

impl StorageApi {
    pub fn is_online(&self) -> bool {
        match self {
            StorageApi::XlStorage(inner) => inner.is_online(),
        }
    }

    pub fn last_conn(&self) -> utils::DateTime {
        match self {
            StorageApi::XlStorage(inner) => inner.last_conn(),
        }
    }

    pub fn is_local(&self) -> bool {
        match self {
            StorageApi::XlStorage(inner) => inner.is_local(),
        }
    }

    pub fn hostname(&self) -> &str {
        match self {
            StorageApi::XlStorage(inner) => inner.hostname(),
        }
    }

    pub fn endpoint(&self) -> &crate::endpoint::Endpoint {
        match self {
            StorageApi::XlStorage(inner) => inner.endpoint(),
        }
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.close(),
        }
    }

    pub async fn get_disk_id(&self) -> anyhow::Result<String> {
        match self {
            StorageApi::XlStorage(inner) => inner.get_disk_id().await,
        }
    }

    pub fn set_disk_id(&mut self, id: String) {
        match self {
            StorageApi::XlStorage(inner) => inner.set_disk_id(id),
        }
    }

    pub async fn healing(&self) -> Option<HealingTracker> {
        match self {
            StorageApi::XlStorage(inner) => inner.healing().await,
        }
    }

    pub async fn disk_info(&self) -> anyhow::Result<DiskInfo> {
        match self {
            StorageApi::XlStorage(inner) => inner.disk_info().await,
        }
    }

    pub async fn namespace_scanner(&self) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.namespace_scanner().await,
        }
    }

    pub async fn make_volume(&self, volume: &str) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.make_volume(volume).await,
        }
    }
    pub async fn make_volumes(&self, volumes: &[&str]) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.make_volumes(volumes).await,
        }
    }
    pub async fn list_volumes(&self) -> anyhow::Result<Vec<VolInfo>> {
        match self {
            StorageApi::XlStorage(inner) => inner.list_volumes().await,
        }
    }
    pub async fn stat_volume(&self, volume: &str) -> anyhow::Result<VolInfo> {
        match self {
            StorageApi::XlStorage(inner) => inner.stat_volume(volume).await,
        }
    }
    pub async fn delete_volume(&self, volume: &str, force_delete: bool) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.delete_volume(volume, force_delete).await,
        }
    }
    pub async fn walk_dir<W: AsyncWrite + Unpin + Send + 'static>(
        &self,
        opts: crate::metacache::WalkDirOptions,
        w: W,
    ) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.walk_dir(opts, w).await,
        }
    }
    pub async fn delete_version(
        &self,
        volume: &str,
        path: &str,
        fi: &FileInfo,
        force_delete_marker: bool,
    ) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => {
                inner
                    .delete_version(volume, path, fi, force_delete_marker)
                    .await
            }
        }
    }
    pub async fn delete_versions(
        &self,
        volume: &str,
        versions: &[&FileInfo],
    ) -> Vec<anyhow::Result<()>> {
        match self {
            StorageApi::XlStorage(inner) => inner.delete_versions(volume, versions).await,
        }
    }
    pub async fn write_metadata(
        &self,
        volume: &str,
        path: &str,
        fi: &FileInfo,
    ) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.write_metadata(volume, path, fi).await,
        }
    }
    pub async fn update_metadata(
        &self,
        volume: &str,
        path: &str,
        fi: &FileInfo,
    ) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.update_metadata(volume, path, fi).await,
        }
    }
    pub async fn read_version(
        &self,
        volume: &str,
        path: &str,
        version_id: &str,
        read_data: bool,
    ) -> anyhow::Result<FileInfo> {
        match self {
            StorageApi::XlStorage(inner) => {
                inner
                    .read_version(volume, path, version_id, read_data)
                    .await
            }
        }
    }
    pub async fn rename_data(
        &self,
        src_volume: &str,
        src_path: &str,
        fi: FileInfo,
        dest_volume: &str,
        dest_path: &str,
    ) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => {
                inner
                    .rename_data(src_volume, src_path, fi, dest_volume, dest_path)
                    .await
            }
        }
    }
    pub async fn list_dir(
        &self,
        volume: &str,
        dir_path: &str,
        count: usize,
    ) -> anyhow::Result<Vec<String>> {
        match self {
            StorageApi::XlStorage(inner) => inner.list_dir(volume, dir_path, count).await,
        }
    }
    pub async fn read_file(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        buf: &mut [u8],
        verifier: Option<bitrot::BitrotVerifier>,
    ) -> anyhow::Result<u64> {
        match self {
            StorageApi::XlStorage(inner) => {
                inner.read_file(volume, path, offset, buf, verifier).await
            }
        }
    }
    pub async fn append_file(&self, volume: &str, path: &str, buf: &[u8]) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.append_file(volume, path, buf).await,
        }
    }
    pub async fn create_file_writer(
        &self,
        volume: &str,
        path: &str,
        file_size: Option<u64>,
    ) -> anyhow::Result<Box<dyn AsyncWrite + Unpin>> {
        match self {
            StorageApi::XlStorage(inner) => inner.create_file_writer(volume, path, file_size).await,
        }
    }
    pub async fn read_file_reader(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        size: u64,
    ) -> anyhow::Result<Box<dyn AsyncRead + Unpin + Send>> {
        match self {
            StorageApi::XlStorage(inner) => {
                inner.read_file_reader(volume, path, offset, size).await
            }
        }
    }
    pub async fn rename_file(
        &self,
        src_volume: &str,
        src_path: &str,
        dest_volume: &str,
        dest_path: &str,
    ) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => {
                inner
                    .rename_file(src_volume, src_path, dest_volume, dest_path)
                    .await
            }
        }
    }
    pub async fn check_parts(&self, volume: &str, path: &str, fi: &FileInfo) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.check_parts(volume, path, fi).await,
        }
    }
    pub async fn check_file(&self, volume: &str, path: &str) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.check_file(volume, path).await,
        }
    }
    pub async fn delete(&self, volume: &str, path: &str, recursive: bool) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.delete(volume, path, recursive).await,
        }
    }
    pub async fn verify_file(&self, volume: &str, path: &str, fi: &FileInfo) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.verify_file(volume, path, fi).await,
        }
    }
    pub async fn write_all(&self, volume: &str, path: &str, data: &[u8]) -> anyhow::Result<()> {
        match self {
            StorageApi::XlStorage(inner) => inner.write_all(volume, path, data).await,
        }
    }
    pub async fn read_all(&self, volume: &str, path: &str) -> anyhow::Result<Vec<u8>> {
        match self {
            StorageApi::XlStorage(inner) => inner.read_all(volume, path).await,
        }
    }
    pub async fn get_disk_location(&self) -> (isize, isize, isize) {
        match self {
            StorageApi::XlStorage(inner) => inner.get_disk_location().await,
        }
    }
    pub fn set_disk_location(&mut self, pool_idx: isize, set_idx: isize, disk_idx: isize) {
        match self {
            StorageApi::XlStorage(inner) => inner.set_disk_location(pool_idx, set_idx, disk_idx),
        }
    }
}
