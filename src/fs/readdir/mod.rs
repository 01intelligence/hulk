#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
mod fs;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
mod readdir;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
mod readdir_impl;
#[cfg(unix)]
mod time;

use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
pub async fn read_dir(dir_path: impl AsRef<Path>) -> std::io::Result<readdir::ReadDir> {
    readdir::read_dir(dir_path).await
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd")))]
pub async fn read_dir(dir_path: impl AsRef<Path>) -> std::io::Result<tokio::fs::ReadDir> {
    tokio::fs::read_dir(dir_path).await
}

#[cfg(test)]
mod tests;
