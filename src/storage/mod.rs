mod datatypes;
pub use datatypes::*;

use crate::{bitrot, utils};

pub enum StorageApi {}

impl StorageApi {
    pub fn is_online(&self) -> bool {
        todo!()
    }

    pub fn last_conn(&self) -> utils::DateTime {
        todo!()
    }

    pub fn is_local(&self) -> bool {
        todo!()
    }

    pub fn hostname(&self) -> &str {
        todo!()
    }

    pub fn endpoint(&self) -> crate::endpoint::Endpoint {
        todo!()
    }

    pub fn close(&mut self) -> anyhow::Result<()> {
        todo!()
    }

    pub fn get_disk_id(&self) -> anyhow::Result<&str> {
        todo!()
    }

    pub fn set_disk_id(&mut self, id: String) {
        todo!()
    }

    pub fn healing(&self) -> HealingTracker {
        todo!()
    }

    pub async fn disk_info(&self) -> anyhow::Result<DiskInfo> {
        todo!()
    }

    pub async fn namespace_scanner(&self) -> anyhow::Result<()> {
        todo!()
    }

    pub async fn make_volume(&self, volume: &str) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn make_volumes(&self, volumes: &[&str]) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn list_volumes(&self) -> anyhow::Result<Vec<VolInfo>> {
        todo!()
    }
    pub async fn stat_volume(&self, volume: &str) -> anyhow::Result<VolInfo> {
        todo!()
    }
    pub async fn delete_volume(&self, volume: &str, force_delete: bool) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn walk_dir(&self) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn delete_version(
        &self,
        volume: &str,
        path: &str,
        file: &FileInfo,
        force_delete_marker: bool,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn delete_versions(
        &self,
        volume: &str,
        versions: &[&FileInfo],
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn write_metadata(
        &self,
        volume: &str,
        path: &str,
        file: &FileInfo,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn update_metadata(
        &self,
        volume: &str,
        path: &str,
        file: &FileInfo,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn read_version(
        &self,
        volume: &str,
        path: &str,
        version_id: &str,
        read_data: bool,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn rename_data(
        &self,
        src_volume: &str,
        src_path: &str,
        file: FileInfo,
        dest_volume: &str,
        dest_path: &str,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn list_dir(
        &self,
        volume: &str,
        dir_path: &str,
        count: usize,
    ) -> anyhow::Result<Vec<String>> {
        todo!()
    }
    pub async fn read_file(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        buf: &[u8],
        verifier: &bitrot::BitrotVerifier,
    ) -> anyhow::Result<u64> {
        todo!()
    }
    pub async fn append_file(&self, volume: &str, path: &str, buf: &[u8]) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn create_file(&self, volume: &str, path: &str, size: u64) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn read_file_stream(
        &self,
        volume: &str,
        path: &str,
        offset: u64,
        size: u64,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn rename_file(
        &self,
        src_volume: &str,
        src_path: &str,
        dest_volume: &str,
        dest_path: &str,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn check_parts(
        &self,
        volume: &str,
        path: &str,
        file: &FileInfo,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn check_file(&self, volume: &str, path: &str) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn delete(&self, volume: &str, path: &str, recursive: bool) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn verify_file(
        &self,
        volume: &str,
        path: &str,
        file: &FileInfo,
    ) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn write_all(&self, volume: &str, path: &str, data: &[u8]) -> anyhow::Result<()> {
        todo!()
    }
    pub async fn read_all(&self, volume: &str, path: &str) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
    pub async fn get_disk_location(&self) -> anyhow::Result<(usize, usize, usize)> {
        todo!()
    }
    pub async fn set_disk_location(
        &mut self,
        pool_idx: usize,
        set_idx: usize,
        disk_idx: usize,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

pub struct HealingTracker {}
