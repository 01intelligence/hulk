use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use scopeguard::defer;
use strum::{Display, EnumCount};

use super::*;
use crate::errors::StorageError;
use crate::globals::{self, GLOBALS};
use crate::utils::{AtomicExt, DateTimeExt};

#[derive(FromPrimitive, EnumCount, Display, Copy, Clone)]
#[repr(u8)]
enum StorageMetric {
    MakeVolBulk,
    MakeVol,
    ListVols,
    StatVol,
    DeleteVol,
    WalkDir,
    ListDir,
    ReadFile,
    AppendFile,
    CreateFile,
    ReadFileStream,
    RenameFile,
    RenameData,
    CheckParts,
    CheckFile,
    Delete,
    DeleteVersions,
    VerifyFile,
    WriteAll,
    DeleteVersion,
    WriteMetadata,
    UpdateMetadata,
    ReadVersion,
    ReadAll,
}

pub(super) struct XlStorageWithCheck {
    storage: XlStorage,
    disk_id: Option<String>,
    api_latencies:
        [Arc<RwLock<(ta::indicators::ExponentialMovingAverage, f64)>>; StorageMetric::COUNT],
    api_calls: [AtomicU64; StorageMetric::COUNT],
}

impl XlStorageWithCheck {
    pub fn new(storage: XlStorage) -> Self {
        Self {
            storage,
            disk_id: None,
            api_latencies: [0u8; StorageMetric::COUNT].map(|_| {
                Arc::new(RwLock::new((
                    ta::indicators::ExponentialMovingAverage::new(30).unwrap(),
                    0f64,
                )))
            }),
            api_calls: [0u8; StorageMetric::COUNT].map(|_| AtomicU64::new(0)),
        }
    }

    fn get_metrics(&self) -> crate::storage::DiskMetrics {
        crate::storage::DiskMetrics {
            api_latencies: self
                .api_latencies
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let v = l.read().unwrap().1;
                    (
                        StorageMetric::from_usize(i).unwrap().to_string(),
                        format!("{:?}", utils::Duration::from_nanos(v as u64)),
                    )
                })
                .collect(),
            api_calls: self
                .api_calls
                .iter()
                .enumerate()
                .map(|(i, n)| (StorageMetric::from_usize(i).unwrap().to_string(), n.get()))
                .collect(),
        }
    }

    fn update_metrics<'a, 'b: 'a>(
        &'b self,
        m: StorageMetric,
        paths: &'a [&str],
    ) -> impl 'a + FnOnce() {
        let start_time = utils::now();
        return move || {
            use ta::Next;
            let duration = start_time.elapsed();
            self.api_calls[m as usize].inc();
            let mut latency = self.api_latencies[m as usize].write().unwrap();
            // Safety: here duration will not overflow.
            latency.1 = latency.0.next(duration.as_nanos() as f64);

            if GLOBALS.trace.subscribers_num() > 0 {
                GLOBALS.trace.publish(crate::admin::TraceInfo {
                    trace_type: crate::admin::TraceType::Storage,
                    node_name: GLOBALS.local_node_name.guard().to_owned(),
                    fn_name: format!("storage.{}", m),
                    time: start_time,
                    storage_stats: Some(crate::admin::TraceStorageStats {
                        path: crate::object::path_join(paths),
                        duration,
                    }),
                    ..Default::default()
                });
            }
        };
    }

    pub fn disk_id(&self) -> &Option<String> {
        &self.disk_id
    }

    pub fn disk_id_mut(&mut self) -> &mut Option<String> {
        &mut self.disk_id
    }

    async fn check_disk_stale(&self) -> anyhow::Result<()> {
        if self.disk_id == None {
            // For empty disk-id we allow the call as the server might be
            // coming up and trying to read format.json or create format.json
            return Ok(());
        }
        let stored_disk_id = self.storage.get_disk_id().await?;
        if self.disk_id == Some(stored_disk_id) {
            return Ok(());
        }
        // not the same disk we remember, take it offline.
        Err(StorageError::DiskNotFound.into())
    }

    pub async fn create_file_writer(
        &self,
        volume: &str,
        path: &str,
        file_size: Option<u64>,
    ) -> anyhow::Result<Box<dyn AsyncWrite + Unpin>> {
        defer! {
            self.update_metrics(StorageMetric::CreateFile, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage
            .create_file_writer(volume, path, file_size)
            .await
    }

    pub async fn make_volume(&self, volume: &str) -> anyhow::Result<()> {
        defer! {
            self.update_metrics(StorageMetric::MakeVol, &[volume])();
        }
        self.check_disk_stale().await?;
        self.storage.make_volume(volume).await
    }

    pub async fn delete_volume(&self, volume: &str, force_delete: bool) -> anyhow::Result<()> {
        defer! {
            self.update_metrics(StorageMetric::DeleteVol, &[volume])();
        }
        self.check_disk_stale().await?;
        self.storage.delete_volume(volume, force_delete).await
    }

    pub async fn stat_volume(&self, volume: &str) -> anyhow::Result<storage::VolInfo> {
        defer! {
            self.update_metrics(StorageMetric::StatVol, &[volume])();
        }
        self.check_disk_stale().await?;
        self.storage.stat_volume(volume).await
    }

    pub async fn list_volumes(&self) -> anyhow::Result<Vec<storage::VolInfo>> {
        defer! {
            self.update_metrics(StorageMetric::ListVols, &["/"])();
        }
        self.check_disk_stale().await?;
        self.storage.list_volumes().await
    }

    pub async fn list_dir(
        &self,
        volume: &str,
        dir_path: &str,
        count: usize,
    ) -> anyhow::Result<Vec<String>> {
        defer! {
            self.update_metrics(StorageMetric::ListDir, &[volume, dir_path])();
        }
        self.check_disk_stale().await?;
        self.storage.list_dir(volume, dir_path, count).await
    }

    pub async fn read_file(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        buf: &mut [u8],
        verifier: Option<crate::bitrot::BitrotVerifier>,
    ) -> anyhow::Result<u64> {
        defer! {
            self.update_metrics(StorageMetric::ReadFile, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage
            .read_file(volume, path, offset, buf, verifier)
            .await
    }

    pub async fn append_file(&self, volume: &str, path: &str, buf: &[u8]) -> anyhow::Result<()> {
        defer! {
            self.update_metrics(StorageMetric::AppendFile, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage.append_file(volume, path, buf).await
    }

    pub async fn rename_file(
        &self,
        src_volume: &str,
        src_path: &str,
        dest_volume: &str,
        dest_path: &str,
    ) -> anyhow::Result<()> {
        defer! {
            self.update_metrics(StorageMetric::RenameFile, &[src_volume, src_path, dest_volume, dest_path])();
        }
        self.check_disk_stale().await?;
        self.storage
            .rename_file(src_volume, src_path, dest_volume, dest_path)
            .await
    }

    pub async fn check_file(&self, volume: &str, path: &str) -> anyhow::Result<()> {
        defer! {
            self.update_metrics(StorageMetric::CheckFile, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage.check_file(volume, path).await
    }

    pub async fn write_all(&self, volume: &str, path: &str, data: &[u8]) -> anyhow::Result<()> {
        defer! {
            self.update_metrics(StorageMetric::WriteAll, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage.write_all(volume, path, data).await
    }

    pub async fn read_version(
        &self,
        volume: &str,
        path: &str,
        version_id: &str,
        read_data: bool,
    ) -> anyhow::Result<FileInfo> {
        defer! {
            self.update_metrics(StorageMetric::ReadVersion, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage
            .read_version(volume, path, version_id, read_data)
            .await
    }

    pub async fn read_all(&self, volume: &str, path: &str) -> anyhow::Result<Vec<u8>> {
        defer! {
            self.update_metrics(StorageMetric::ReadAll, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage.read_all(volume, path).await
    }

    pub async fn delete(&self, volume: &str, path: &str, recursive: bool) -> anyhow::Result<()> {
        defer! {
            self.update_metrics(StorageMetric::Delete, &[volume, path])();
        }
        self.check_disk_stale().await?;
        self.storage.delete(volume, path, recursive).await
    }
}
