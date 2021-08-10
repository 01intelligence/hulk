use std::convert::TryFrom;
use std::io::ErrorKind;
use std::path::Path;

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
        return match mkdir_all(path.as_ref(), mode).await {
            Err(err) => {
                // Retry only for the first retryable error.
                if err.kind() == ErrorKind::NotFound && first {
                    first = false;
                    continue;
                }
                Err(err)
            }
            Ok(_) => Ok(()),
        };
    }
}

async fn reliable_rename(
    src_path: impl AsRef<Path>,
    dst_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let src_path_str = src_path.as_ref().to_string_lossy();
    let dst_path_str = dst_path.as_ref().to_string_lossy();
    let _ = check_path_length(src_path_str.as_ref())?;
    let _ = check_path_length(dst_path_str.as_ref())?;
    if let Err(err) = reliable_rename_inner(src_path.as_ref(), dst_path.as_ref()).await {
        return if err_not_found(&err) {
            Err(StorageError::FileNotFound.into())
        } else if err_not_dir(&err) {
            Err(StorageError::FileAccessDenied.into())
        } else if err_cross_device(&err) {
            Err(
                StorageError::CrossDeviceLink(src_path_str.to_string(), dst_path_str.to_string())
                    .into(),
            )
        } else if err_already_exists(&err) {
            Err(StorageError::IsNotRegular.into())
        } else {
            Err(err.into())
        };
    }
    Ok(())
}

async fn reliable_rename_inner(
    src_path: impl AsRef<Path>,
    dst_path: impl AsRef<Path>,
) -> std::io::Result<()> {
    if let Some(dst_dir) = dst_path.as_ref().parent() {
        let _ = reliable_mkdir_all_inner(dst_dir, 0o777).await?;
    }

    let mut first = true;
    loop {
        return match rename(src_path.as_ref(), dst_path.as_ref()).await {
            Err(err) => {
                // Retry only for the first retryable error.
                if err.kind() == ErrorKind::NotFound && first {
                    first = false;
                    continue;
                }
                Err(err)
            }
            Ok(_) => Ok(()),
        };
    }
}

async fn reliable_remove_all(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path_str = path.as_ref().to_string_lossy();
    let _ = check_path_length(path_str.as_ref())?;

    if let Err(err) = reliable_remove_all_inner(path).await {
        return if err_not_dir(&err) {
            Err(StorageError::FileAccessDenied.into())
        } else if err_not_found(&err) {
            Err(StorageError::FileAccessDenied.into())
        } else {
            Err(err.into())
        };
    }
    Ok(())
}

async fn reliable_remove_all_inner(path: impl AsRef<Path>) -> std::io::Result<()> {
    let mut first = true;
    loop {
        return match remove_all(path.as_ref()).await {
            Err(err) => {
                // Retry only for the first retryable error.
                if err_dir_not_empty(&err) && first {
                    first = false;
                    continue;
                }
                Err(err)
            }
            Ok(_) => Ok(()),
        };
    }
}
