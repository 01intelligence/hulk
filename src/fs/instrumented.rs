use std::path::Path;

use tokio::fs;

pub fn access(path: impl AsRef<Path>) -> std::io::Result<()> {
    use faccess::{AccessMode, PathExt};
    path.as_ref().access(AccessMode::READ | AccessMode::WRITE)
}

pub async fn mkdir_all(path: impl AsRef<Path>, mode: u32) -> std::io::Result<()> {
    let mut b = fs::DirBuilder::new();
    // TODO: windows
    #[cfg(unix)]
    b.mode(mode);
    b.recursive(true).create(path).await
}
