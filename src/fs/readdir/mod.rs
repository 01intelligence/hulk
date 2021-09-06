#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
mod readdir;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
mod readdir_impl;

use std::fs::{FileType, Metadata};
use std::io;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::task::{Context, Poll};

use futures_core::Stream;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
use readdir::{DirEntry, ReadDir};
#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd")))]
use tokio::fs::{DirEntry, ReadDir};

use crate::fs::{err_not_found, err_too_many_symlinks};
use crate::prelude::*;

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
pub async fn read_dir(dir_path: impl AsRef<Path>) -> std::io::Result<ReadDir> {
    readdir::read_dir(dir_path).await
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd")))]
pub async fn read_dir(dir_path: impl AsRef<Path>) -> std::io::Result<ReadDir> {
    tokio::fs::read_dir(dir_path).await
}

pub struct ReadDirEntries<P: AsRef<Path>>(P, Option<ReadDir>);

impl<P: AsRef<Path>> ReadDirEntries<P> {
    pub fn new(dir_path: P) -> ReadDirEntries<P> {
        ReadDirEntries(dir_path, None)
    }

    pub async fn next_entry(&mut self) -> io::Result<Option<(String, FileType)>> {
        let stream = if self.1.is_none() {
            let stream = read_dir(self.0.as_ref()).await?;
            self.1.insert(stream)
        } else {
            self.1.as_mut().unwrap()
        };
        while let Some(entry) = stream.next_entry().await? {
            let mut typ = entry.file_type().await?;
            let path: crate::utils::PathBuf = entry
                .path()
                .try_into()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

            if typ.is_symlink() {
                // Traverse symlinks.
                let meta = match crate::fs::metadata(&path).await {
                    Ok(meta) => meta,
                    Err(err) => {
                        // It got deleted in the meantime, not found
                        // or returns too many symlinks, ignore this
                        // file/directory.
                        if err_not_found(&err) && err_too_many_symlinks(&err) {
                            continue;
                        }
                        return Err(err.into());
                    }
                };
                // Ignore symlinked directories.
                if meta.is_dir() {
                    continue;
                }
                typ = meta.file_type();
            }

            let name = path
                .to_str()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::Other, "PathBuf contains invalid UTF-8")
                })?
                .to_owned();
            let name = if typ.is_file() {
                name
            } else if typ.is_dir() {
                name + crate::globals::SLASH_SEPARATOR
            } else {
                continue;
            };

            return Ok(Some((name, typ)));
        }

        Ok(None)
    }
}

pub async fn read_dir_entries(dir_path: impl AsRef<Path>) -> std::io::Result<Vec<String>> {
    read_dir_entries_n(dir_path, usize::MAX).await
}

pub async fn read_dir_entries_n(
    dir_path: impl AsRef<Path>,
    mut n: usize,
) -> std::io::Result<Vec<String>> {
    let mut entries = Vec::new();
    let mut stream = ReadDirEntries::new(dir_path);
    while let Some((name, _)) = stream.next_entry().await? {
        if n == 0 {
            break;
        }
        n -= 1;
        entries.push(name);
    }
    Ok(entries)
}

pub async fn is_dir_empty(dir_path: impl AsRef<Path>) -> bool {
    match read_dir_entries_n(dir_path, 1).await {
        Ok(entries) => entries.len() == 1,
        Err(_) => false,
    }
}

pub async fn asyncify<F, T>(f: F) -> io::Result<T>
where
    F: FnOnce() -> io::Result<T> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::Other,
            "background task failed",
        )),
    }
}

#[cfg(test)]
mod tests;
