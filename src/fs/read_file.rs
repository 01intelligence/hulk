use tokio::io::AsyncReadExt;

use super::*;
use crate::utils::Path;

/// Read whole file content, with NOATIME flag.
pub async fn read_file<P: AsRef<Path>>(name: P) -> std::io::Result<Vec<u8>> {
    let mut file = OpenOptions::new()
        .read(true)
        .no_atime()
        .open(name.as_ref())
        .await?;
    let mut data = Vec::new();
    let _ = file.read_to_end(&mut data).await?;
    Ok(data)
}
