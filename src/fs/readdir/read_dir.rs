use std::ffi::OsString;
use std::fs::{FileType, Metadata};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_core::ready;
use futures_util::future::poll_fn;
use tokio::task::{spawn_blocking, JoinHandle};

pub async fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    let path = path.as_ref().to_owned();
    let std = asyncify(|| std::fs::read_dir(path)).await?;

    Ok(ReadDir(State::Idle(Some(std))))
}

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ReadDir(State);

#[derive(Debug)]
enum State {
    Idle(Option<std::fs::ReadDir>),
    Pending(JoinHandle<(Option<io::Result<std::fs::DirEntry>>, std::fs::ReadDir)>),
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
        use std::os::unix::fs::DirEntryExt;
        self.as_inner().ino()
    }
}

#[derive(Debug)]
pub struct DirEntry(Arc<std::fs::DirEntry>);

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.0.path()
    }

    pub fn file_name(&self) -> OsString {
        self.0.file_name()
    }

    pub async fn metadata(&self) -> io::Result<Metadata> {
        let std = self.0.clone();
        asyncify(move || std.metadata()).await
    }

    pub async fn file_type(&self) -> io::Result<FileType> {
        let std = self.0.clone();
        asyncify(move || std.file_type()).await
    }

    #[cfg(unix)]
    pub(super) fn as_inner(&self) -> &std::fs::DirEntry {
        &self.0
    }
}

async fn asyncify<F, T>(f: F) -> io::Result<T>
where
    F: FnOnce() -> io::Result<T> + Send + 'static,
    T: Send + 'static,
{
    match spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::Other,
            "background task failed",
        )),
    }
}
