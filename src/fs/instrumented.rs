//! Instrument metrics for used filesystem operations from [`tokio::fs`].

use tokio::fs;

use crate::feature;
use crate::prelude::*;
use crate::utils::{DateTime, Path, PathBuf};

pub async fn mkdir_all(path: impl AsRef<Path>, mode: u32) -> std::io::Result<()> {
    let mut b = fs::DirBuilder::new();
    // TODO: windows
    #[cfg(unix)]
    b.mode(mode);
    b.recursive(true).create(path.as_ref().as_std_path()).await
}

pub async fn rename(src_path: impl AsRef<Path>, dst_path: impl AsRef<Path>) -> std::io::Result<()> {
    fs::rename(src_path.as_ref(), dst_path.as_ref()).await
}

pub async fn remove(path: impl AsRef<Path>) -> std::io::Result<()> {
    let meta = fs::metadata(path.as_ref()).await?;
    if meta.is_dir() {
        fs::remove_dir(path.as_ref()).await
    } else {
        fs::remove_file(path.as_ref()).await
    }
}

pub async fn remove_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    let meta = fs::metadata(path.as_ref()).await?;
    if meta.is_dir() {
        fs::remove_dir_all(path.as_ref()).await
    } else {
        fs::remove_file(path.as_ref()).await
    }
}

pub async fn access(path: impl AsRef<Path>) -> std::io::Result<()> {
    use faccess::{AccessMode, PathExt};
    let path = path.as_ref().to_owned();
    super::asyncify(move || path.access(AccessMode::READ | AccessMode::WRITE)).await
}

pub async fn metadata(path: impl AsRef<Path>) -> std::io::Result<std::fs::Metadata> {
    fs::metadata(path.as_ref()).await
}

pub async fn canonicalize(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    fs::canonicalize(path.as_ref())
        .await
        .map(|p| p.try_into().unwrap())
}

pub trait MetadataExt {
    fn modified_at(&self) -> crate::utils::DateTime;
    fn accessed_at(&self) -> crate::utils::DateTime;
    fn created_at(&self) -> crate::utils::DateTime;
}

impl MetadataExt for std::fs::Metadata {
    fn modified_at(&self) -> DateTime {
        // Available on Unix/Windows, so unwrap is safe.
        self.modified().unwrap().into()
    }

    fn accessed_at(&self) -> DateTime {
        // Available on Unix/Windows, so unwrap is safe.
        self.accessed().unwrap().into()
    }

    fn created_at(&self) -> DateTime {
        // Available on Unix/Windows, so unwrap is safe.
        self.created().unwrap().into()
    }
}
