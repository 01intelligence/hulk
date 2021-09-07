use crate::utils::{Path, PathBuf};

pub async fn temp_dir() -> PathBuf {
    PathBuf::from_path_buf(std::env::temp_dir()).expect("temp path")
}

pub async fn create_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
    tokio::fs::create_dir(path.as_ref().as_std_path()).await
}

pub async fn remove_dir_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    tokio::fs::remove_dir_all(path.as_ref().as_std_path()).await
}
