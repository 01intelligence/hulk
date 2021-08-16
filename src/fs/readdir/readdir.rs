use std::ffi::OsString;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_core::ready;
use futures_util::future::poll_fn;
use tokio::task::{spawn_blocking, JoinHandle};

use super::{asyncify, readdir_impl};

pub async fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    let path = path.as_ref().to_owned();
    let read_dir = asyncify(|| readdir_impl::readdir(path)).await?;

    Ok(ReadDir(State::Idle(Some(read_dir))))
}

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ReadDir(State);

#[derive(Debug)]
enum State {
    Idle(Option<readdir_impl::ReadDir>),
    Pending(
        JoinHandle<(
            Option<io::Result<readdir_impl::DirEntry>>,
            readdir_impl::ReadDir,
        )>,
    ),
}

impl ReadDir {
    /// Returns the next entry in the directory stream.
    pub async fn next_entry(&mut self) -> io::Result<Option<DirEntry>> {
        poll_fn(|cx| self.poll_next_entry(cx)).await
    }

    pub fn poll_next_entry(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<Option<DirEntry>>> {
        loop {
            match self.0 {
                State::Idle(ref mut std) => {
                    let mut std = std.take().unwrap();

                    self.0 = State::Pending(spawn_blocking(move || {
                        let ret = std.next();
                        (ret, std)
                    }));
                }
                State::Pending(ref mut rx) => {
                    let (ret, std) = ready!(Pin::new(rx).poll(cx))?;
                    self.0 = State::Idle(Some(std));

                    let ret = match ret {
                        Some(Ok(std)) => Ok(Some(DirEntry(Arc::new(std)))),
                        Some(Err(e)) => Err(e),
                        None => Ok(None),
                    };

                    return Poll::Ready(ret);
                }
            }
        }
    }
}

#[cfg(unix)]
impl DirEntry {
    pub fn ino(&self) -> u64 {
        self.as_inner().ino()
    }
}

#[derive(Debug)]
pub struct DirEntry(Arc<readdir_impl::DirEntry>);

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.0.path()
    }

    pub fn file_name(&self) -> OsString {
        self.0.file_name()
    }

    pub async fn metadata(&self) -> io::Result<std::fs::Metadata> {
        let std = self.0.clone();
        asyncify(move || std.metadata().map(|m| Metadata(m).to_std())).await
    }

    pub async fn file_type(&self) -> io::Result<std::fs::FileType> {
        let std = self.0.clone();
        asyncify(move || std.file_type().map(|f| FileType(f).to_std())).await
    }

    #[cfg(unix)]
    pub(super) fn as_inner(&self) -> &readdir_impl::DirEntry {
        &self.0
    }
}

pub struct Metadata(readdir_impl::FileAttr);

pub struct FileType(readdir_impl::FileType);

impl Metadata {
    pub fn to_std(self) -> std::fs::Metadata {
        unsafe { std::mem::transmute(self) }
    }
}

impl FileType {
    pub fn to_std(self) -> std::fs::FileType {
        unsafe { std::mem::transmute(self) }
    }
}
