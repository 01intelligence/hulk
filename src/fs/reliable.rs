use std::convert::TryFrom;
use std::io::{self, ErrorKind};
use std::path::Path;

use tokio::fs;

use super::*;
use crate::errors::StorageError;

pub async fn reliable_mkdir_all(path: impl AsRef<Path>, mode: u32) -> anyhow::Result<()> {
    let path_str = path.as_ref().to_string_lossy();
    let _ = check_path_length(path_str.as_ref())?;

    if let Err(err) = reliable_mkdir_all_inner(path, mode).await {
        return if err_not_dir(&err) {
            Err(StorageError::FileAccessDenied.into())
        } else if err_not_found(&err) {
            Err(StorageError::FileAccessDenied.into())
        } else {
            match StorageError::try_from(err) {
                Ok(err) => Err(err.into()),
                Err(err) => Err(err.into()),
            }
        };
    }
    Ok(())
}

async fn reliable_mkdir_all_inner(path: impl AsRef<Path>, mode: u32) -> std::io::Result<()> {
    let mut first = true;
    loop {
        match mkdir_all(path.as_ref(), mode).await {
            Err(err) => {
                // Retry only for the first retryable error.
                if err.kind() == ErrorKind::NotFound && first {
                    first = false;
                    continue;
                }
                return Err(err);
            }
            Ok(_) => return Ok(()),
        }
    }
}
