use std::future::Future;
use std::io::ErrorKind;
use std::pin::Pin;

use tokio::fs::File;

use crate::prelude::{Deref, DerefMut};
use crate::utils::Path;

pub const DIRECTIO_ALIGN_SIZE: usize = 4096;

pub trait OpenOptionsDirectIo {
    fn open_direct_io<'a>(
        &'a mut self,
        path: impl AsRef<Path> + Send + Sync + 'a,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<File>> + Send + Sync + 'a>>;
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
impl OpenOptionsDirectIo for super::OpenOptions {
    fn open_direct_io<'a>(
        &'a mut self,
        path: impl AsRef<Path> + Send + Sync + 'a,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<File>> + Send + Sync + 'a>> {
        Box::pin(self.append_custom_flags(libc::O_DIRECT).open(path))
    }
}

#[cfg(target_os = "macos")]
impl OpenOptionsDirectIo for super::OpenOptions {
    fn open_direct_io<'a>(
        &'a mut self,
        path: impl AsRef<Path> + Send + Sync + 'a,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<File>> + Send + Sync + 'a>> {
        Box::pin(async move {
            use std::os::unix::io::AsRawFd;
            let file = self.open(path).await?;
            // F_NOCACHE: Turns data caching off/on.
            // A non-zero value in arg turns data caching off.
            // A value of zero in arg turns data caching on.
            let res = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_NOCACHE, 1) };
            let _ = nix::errno::Errno::result(res)
                .map_err(|e| std::io::Error::from(e.as_errno().unwrap()))?;
            Ok(file)
        })
    }
}

#[cfg(target_family = "windows")]
impl OpenOptionsDirectIo for super::OpenOptions {
    fn open_direct_io<'a>(
        &'a mut self,
        path: impl AsRef<Path> + Send + Sync + 'a,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<File>> + Send + Sync + 'a>> {
        // Do not support O_DIRECT on Windows.
        Box::pin(self.open(path))
    }
}

pub trait FileDirectIo {
    fn direct_io(&self, enable: bool) -> std::io::Result<()>;
    fn enable_direct_io(&self) -> std::io::Result<()> {
        self.direct_io(true)
    }
    fn disable_direct_io(&self) -> std::io::Result<()> {
        self.direct_io(false)
    }
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
impl FileDirectIo for std::fs::File {
    fn direct_io(&self, enable: bool) -> std::io::Result<()> {
        use std::os::unix::io::AsRawFd;

        use nix::fcntl::*;
        let fd = self.as_raw_fd();
        let flag = fcntl(fd, FcntlArg::F_GETFL)
            .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?;
        let mut flag = OFlag::from_bits(flag)
            .ok_or_else(|| std::io::Error::new(ErrorKind::Other, "invalid OFlag"))?;
        if enable {
            flag.insert(OFlag::O_DIRECT);
        } else {
            flag.remove(OFlag::O_DIRECT);
        }
        let _ = fcntl(fd, FcntlArg::F_SETFL(flag))
            .map_err(|err| std::io::Error::new(ErrorKind::Other, err));
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl FileDirectIo for std::fs::File {
    fn direct_io(&self, enable: bool) -> std::io::Result<()> {
        use std::os::unix::io::AsRawFd;
        let res = unsafe {
            libc::fcntl(
                self.as_raw_fd(),
                libc::F_NOCACHE,
                if enable { 1 } else { 0 },
            )
        };
        if let Err(err) = nix::errno::Errno::result(res) {
            return Err(std::io::Error::new(ErrorKind::Other, err));
        }
        Ok(())
    }
}

#[cfg(target_family = "windows")]
impl FileDirectIo for std::fs::File {
    fn direct_io(&self, _enable: bool) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct AlignedBlock {
    ptr: std::ptr::NonNull<u8>,
    layout: std::alloc::Layout,
}

impl AlignedBlock {
    pub fn new(block_size: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(block_size, DIRECTIO_ALIGN_SIZE).unwrap();
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

unsafe impl<const SIZE: usize> Send for SizedAlignedBlock<SIZE> {}
unsafe impl<const SIZE: usize> Sync for SizedAlignedBlock<SIZE> {}

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
