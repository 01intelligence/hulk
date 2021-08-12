mod readdir;
#[cfg(unix)]
mod readdir_impl;

use std::path::Path;

use tokio::fs;

pub async fn read_dir(dir_path: impl AsRef<Path>) {
    // fs::read_dir();
}
