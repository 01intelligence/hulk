use async_trait::async_trait;
use tokio::fs::File;

use crate::prelude::{Deref, DerefMut};
use crate::utils::Path;

const ALIGN_SIZE: usize = 4096;

#[async_trait]
pub trait OpenOptionsDirectIo {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + Sync + 'async_trait,
    ) -> std::io::Result<File>;
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
#[async_trait]
impl OpenOptionsDirectIo for super::OpenOptions {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + Sync + 'async_trait,
    ) -> std::io::Result<File> {
        let file = self.append_custom_flags(libc::O_DIRECT).open(path).await?;
        Ok(file)
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl OpenOptionsDirectIo for super::OpenOptions {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + Sync + 'async_trait,
    ) -> std::io::Result<File> {
        use std::os::unix::io::AsRawFd;
        let file = self.open(path).await?;
        // F_NOCACHE: Turns data caching off/on.
        // A non-zero value in arg turns data caching off.
        // A value of zero in arg turns data caching on.
        let res = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_NOCACHE, 1) };
        let _ = nix::errno::Errno::result(res)
            .map_err(|e| std::io::Error::from(e.as_errno().unwrap()))?;
        Ok(file)
    }
}

#[cfg(target_family = "windows")]
#[async_trait]
impl OpenOptionsDirectIo for super::OpenOptions {
    async fn open_direct_io(
        &mut self,
        path: impl AsRef<Path> + Send + Sync + 'async_trait,
    ) -> std::io::Result<File> {
        // Do not support O_DIRECT on Windows.
        let file = self.open(path).await?;
        Ok(file)
    }
}

pub trait FileDirectIo {
    fn direct_io(&self, enable: bool) -> anyhow::Result<()>;
    fn enable_direct_io(&self) -> anyhow::Result<()> {
        self.direct_io(true)
    }
    fn disable_direct_io(&self) -> anyhow::Result<()> {
        self.direct_io(false)
    }
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
impl FileDirectIo for File {
    fn direct_io(&self, enable: bool) -> anyhow::Result<()> {
        use std::os::unix::io::AsRawFd;

        use nix::fcntl::*;
        let fd = self.as_raw_fd();
        let flag = fcntl(fd, FcntlArg::F_GETFL)?;
        let mut flag = OFlag::from_bits(flag).ok_or_else(|| anyhow::anyhow!("invalid OFlag"))?;
        if enable {
            flag.insert(OFlag::O_DIRECT);
        } else {
            flag.remove(OFlag::O_DIRECT);
        }
        let _ = fcntl(fd, FcntlArg::F_SETFL(flag))?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl FileDirectIo for File {
    fn direct_io(&self, enable: bool) -> anyhow::Result<()> {
        use std::os::unix::io::AsRawFd;
        let res = unsafe {
            libc::fcntl(
                self.as_raw_fd(),
                libc::F_NOCACHE,
                if enable { 1 } else { 0 },
            )
        };
        let _ = nix::errno::Errno::result(res)?;
        Ok(())
    }
}

#[cfg(target_family = "windows")]
impl FileDirectIo for File {
    fn direct_io(&self, _enable: bool) -> anyhow::Result<()> {
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

unsafe impl Send for AlignedBlock {}
unsafe impl Sync for AlignedBlock {}

impl Deref for AlignedBlock {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ref(), self.layout.size()) }
    }
}

impl DerefMut for AlignedBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_mut(), self.layout.size()) }
    }
}

pub struct SizedAlignedBlock<const SIZE: usize>(AlignedBlock);

impl<const SIZE: usize> Default for SizedAlignedBlock<SIZE> {
    fn default() -> Self {
        SizedAlignedBlock(AlignedBlock::new(SIZE))
    }
}

impl<const SIZE: usize> Deref for SizedAlignedBlock<SIZE> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const SIZE: usize> DerefMut for SizedAlignedBlock<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
