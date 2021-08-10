use std::path::Path;

use tokio::fs;

pub async fn mkdir_all(path: impl AsRef<Path>, mode: u32) -> std::io::Result<()> {
    let mut b = fs::DirBuilder::new();
    // TODO: windows
    #[cfg(unix)]
    b.mode(mode);
    b.recursive(true).create(path).await
}

pub async fn rename(src_path: impl AsRef<Path>, dst_path: impl AsRef<Path>) -> std::io::Result<()> {
    fs::rename(src_path, dst_path).await
}

pub async fn remove_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    let meta = fs::metadata(path.as_ref()).await?;
    if meta.is_dir() {
        fs::remove_dir_all(path).await
    } else {
        fs::remove_file(path).await
    }
}

pub fn access(path: impl AsRef<Path>) -> std::io::Result<()> {
    use faccess::{AccessMode, PathExt};
    path.as_ref().access(AccessMode::READ | AccessMode::WRITE)
}
