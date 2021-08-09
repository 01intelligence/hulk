use std::path::Path;

use async_trait::async_trait;
use tokio::fs;
use tokio::fs::File;

const ALIGN_SIZE: usize = 4096;

#[async_trait]
pub trait OpenOptionsDirectIo {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + 'static,
    ) -> anyhow::Result<fs::File>;
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
#[async_trait]
impl OpenOptionsDirectIo for fs::OpenOptions {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + 'static,
    ) -> anyhow::Result<fs::File> {
        let file = self.custom_flags(libc::O_DIRECT).open(path).await?;
        Ok(file)
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl OpenOptionsDirectIo for fs::OpenOptions {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + 'static,
    ) -> anyhow::Result<fs::File> {
        use std::os::unix::io::AsRawFd;
        let file = self.custom_flags(libc::O_DIRECT).open(path).await?;
        // F_NOCACHE: Turns data caching off/on.
        // A non-zero value in arg turns data caching off.
        // A value of zero in arg turns data caching on.
        let res = libc::fcntl(file.as_raw_fd(), libc::F_NOCACHE, 1);
        let _ = nix::error::Errno::result(res)?;
        file
    }
}

#[cfg(target_family = "windows")]
#[async_trait]
impl OpenOptionsDirectIo for fs::OpenOptions {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + 'static,
    ) -> anyhow::Result<File> {
        // Do not support O_DIRECT on Windows.
        let file = self.open(path).await?;
        Ok(file)
    }
}

pub trait FileDirectIo {
    fn disable_direct_io(&self) -> anyhow::Result<()>;
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
impl FileDirectIo for fs::File {
    fn disable_direct_io(&self) -> anyhow::Result<()> {
        use std::os::unix::io::AsRawFd;

        use nix::fcntl::*;
        let fd = self.as_raw_fd();
        let flag = fcntl(fd, FcntlArg::F_GETFL)?;
        let mut flag = OFlag::from_bits(flag).ok_or_else(|| anyhow::anyhow!("invalid OFlag"))?;
        flag.remove(OFlag::O_DIRECT);
        let _ = fcntl(fd, FcntlArg::F_SETFL(flag))?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl FileDirectIo for fs::File {
    fn disable_direct_io(&self) -> anyhow::Result<()> {
        use std::os::unix::io::AsRawFd;
        let res = libc::fcntl(self.as_raw_fd(), libc::F_NOCACHE, 0);
        let _ = nix::error::Errno::result(res)?;
        Ok(())
    }
}

#[cfg(target_family = "windows")]
impl FileDirectIo for fs::File {
    fn disable_direct_io(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct AlignedBlock {
    ptr: std::ptr::NonNull<u8>,
    layout: std::alloc::Layout,
}

impl AlignedBlock {
    pub fn new(block_size: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(block_size, ALIGN_SIZE).unwrap();
        Self {
            ptr: std::ptr::NonNull::new(unsafe { std::alloc::alloc(layout) }).unwrap(),
            layout,
        }
    }
}

impl Drop for AlignedBlock {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

impl AsRef<[u8]> for AlignedBlock {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ref(), self.layout.size()) }
    }
}

impl AsMut<[u8]> for AlignedBlock {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_mut(), self.layout.size()) }
    }
}
