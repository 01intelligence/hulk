use std::ffi::{CStr, CString, OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{io, mem, ptr};

#[cfg(any(all(target_os = "linux", target_env = "gnu"), target_os = "macos",))]
use libc::c_char;
/*
pub struct dirent64 {
    pub d_ino: ino64_t,
    pub d_off: off64_t,
    pub d_reclen: c_ushort,
    pub d_type: c_uchar,
    pub d_name: [c_char; 256],
}
*/
use libc::{c_int, dirent64, mode_t};
#[cfg(any(target_os = "linux"))]
use libc::{fstatat64, lstat64, off64_t, stat64};
#[cfg(not(any(target_os = "linux")))]
use libc::{lstat as lstat64, off_t as off64_t, stat as stat64};
use thiserror::Error;

use crate::sys::cvt;
use crate::sys::time::SystemTime;
#[cfg(any(all(target_os = "linux", target_env = "gnu"), target_os = "macos",))]
use crate::sys::weak::syscall;

// FIXME: This should be available on Linux with all `target_env`.
// But currently only glibc exposes `statx` fn and structs.
// We don't want to import unverified raw C structs here directly.
// https://github.com/rust-lang/rust/pull/67774
macro_rules! cfg_has_statx {
    ({ $($then_tt:tt)* } else { $($else_tt:tt)* }) => {
        cfg_if::cfg_if! {
            if #[cfg(all(target_os = "linux", target_env = "gnu"))] {
                $($then_tt)*
            } else {
                $($else_tt)*
            }
        }
    };
    ($($block_inner:tt)*) => {
        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        {
            $($block_inner)*
        }
    };
}

cfg_has_statx! {{
    #[derive(Clone)]
    pub struct FileAttr {
        stat: stat64,
        statx_extra_fields: Option<StatxExtraFields>,
    }

    #[derive(Clone)]
    struct StatxExtraFields {
        // This is needed to check if btime is supported by the filesystem.
        stx_mask: u32,
        stx_btime: libc::statx_timestamp,
    }

    // We prefer `statx` on Linux if available, which contains file creation time.
    // Default `stat64` contains no creation time.
    unsafe fn try_statx(
        fd: c_int,
        path: *const c_char,
        flags: i32,
        mask: u32,
    ) -> Option<io::Result<FileAttr>> {
        use std::sync::atomic::{AtomicU8, Ordering};

        // Linux kernel prior to 4.11 or glibc prior to glibc 2.28 don't support `statx`
        // We store the availability in global to avoid unnecessary syscalls.
        // 0: Unknown
        // 1: Not available
        // 2: Available
        static STATX_STATE: AtomicU8 = AtomicU8::new(0);
        syscall! {
            fn statx(
                fd: c_int,
                pathname: *const c_char,
                flags: c_int,
                mask: libc::c_uint,
                statxbuf: *mut libc::statx
            ) -> c_int
        }

        match STATX_STATE.load(Ordering::Relaxed) {
            0 => {
                // It is a trick to call `statx` with null pointers to check if the syscall
                // is available. According to the manual, it is expected to fail with EFAULT.
                // We do this mainly for performance, since it is nearly hundreds times
                // faster than a normal successful call.
                let err = cvt(statx(0, ptr::null(), 0, libc::STATX_ALL, ptr::null_mut()))
                    .err()
                    .and_then(|e| e.raw_os_error());
                // We don't check `err == Some(libc::ENOSYS)` because the syscall may be limited
                // and returns `EPERM`. Listing all possible errors seems not a good idea.
                // See: https://github.com/rust-lang/rust/issues/65662
                if err != Some(libc::EFAULT) {
                    STATX_STATE.store(1, Ordering::Relaxed);
                    return None;
                }
                STATX_STATE.store(2, Ordering::Relaxed);
            }
            1 => return None,
            _ => {}
        }

        let mut buf: libc::statx = mem::zeroed();
        if let Err(err) = cvt(statx(fd, path, flags, mask, &mut buf)) {
            return Some(Err(err));
        }

        // We cannot fill `stat64` exhaustively because of private padding fields.
        let mut stat: stat64 = mem::zeroed();
        // `c_ulong` on gnu-mips, `dev_t` otherwise
        stat.st_dev = libc::makedev(buf.stx_dev_major, buf.stx_dev_minor) as _;
        stat.st_ino = buf.stx_ino as libc::ino64_t;
        stat.st_nlink = buf.stx_nlink as libc::nlink_t;
        stat.st_mode = buf.stx_mode as libc::mode_t;
        stat.st_uid = buf.stx_uid as libc::uid_t;
        stat.st_gid = buf.stx_gid as libc::gid_t;
        stat.st_rdev = libc::makedev(buf.stx_rdev_major, buf.stx_rdev_minor) as _;
        stat.st_size = buf.stx_size as off64_t;
        stat.st_blksize = buf.stx_blksize as libc::blksize_t;
        stat.st_blocks = buf.stx_blocks as libc::blkcnt64_t;
        stat.st_atime = buf.stx_atime.tv_sec as libc::time_t;
        // `i64` on gnu-x86_64-x32, `c_ulong` otherwise.
        stat.st_atime_nsec = buf.stx_atime.tv_nsec as _;
        stat.st_mtime = buf.stx_mtime.tv_sec as libc::time_t;
        stat.st_mtime_nsec = buf.stx_mtime.tv_nsec as _;
        stat.st_ctime = buf.stx_ctime.tv_sec as libc::time_t;
        stat.st_ctime_nsec = buf.stx_ctime.tv_nsec as _;

        let extra = StatxExtraFields {
            stx_mask: buf.stx_mask,
            stx_btime: buf.stx_btime,
        };

        Some(Ok(FileAttr { stat, statx_extra_fields: Some(extra) }))
    }

} else {
    #[derive(Clone)]
    pub struct FileAttr {
        stat: stat64,
    }
}}

struct InnerReadDir {
    fd: c_int,
    root: PathBuf,
}

pub struct ReadDir {
    inner: Arc<InnerReadDir>,
}

pub struct DirEntry {
    entry: dirent64,
    dir: Arc<InnerReadDir>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermissions {
    mode: mode_t,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FileType {
    mode: mode_t,
}

cfg_has_statx! {{
    impl FileAttr {
        fn from_stat64(stat: stat64) -> Self {
            Self { stat, statx_extra_fields: None }
        }
    }
} else {
    impl FileAttr {
        fn from_stat64(stat: stat64) -> Self {
            Self { stat }
        }
    }
}}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.stat.st_size as u64
    }
    pub fn perm(&self) -> FilePermissions {
        FilePermissions {
            mode: (self.stat.st_mode as mode_t),
        }
    }

    pub fn file_type(&self) -> FileType {
        FileType {
            mode: self.stat.st_mode as mode_t,
        }
    }
}

impl FileAttr {
    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from(libc::timespec {
            tv_sec: self.stat.st_mtime as libc::time_t,
            tv_nsec: self.stat.st_mtime_nsec as _,
        }))
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from(libc::timespec {
            tv_sec: self.stat.st_atime as libc::time_t,
            tv_nsec: self.stat.st_atime_nsec as _,
        }))
    }

    #[cfg(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "macos",
        target_os = "ios"
    ))]
    pub fn created(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from(libc::timespec {
            tv_sec: self.stat.st_birthtime as libc::time_t,
            tv_nsec: self.stat.st_birthtime_nsec as libc::c_long,
        }))
    }

    #[cfg(not(any(
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "macos",
        target_os = "ios"
    )))]
    pub fn created(&self) -> io::Result<SystemTime> {
        cfg_has_statx! {
            if let Some(ext) = &self.statx_extra_fields {
                return if (ext.stx_mask & libc::STATX_BTIME) != 0 {
                    Ok(SystemTime::from(libc::timespec {
                        tv_sec: ext.stx_btime.tv_sec as libc::time_t,
                        tv_nsec: ext.stx_btime.tv_nsec as _,
                    }))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Uncategorized,
                        Error::CreationTimeUnavailableForFilesystem,
                    ))
                };
            }
        }

        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            Error::CreationTimeUnavailableForPlatform,
        ))
    }
}

#[derive(Error, Debug)]
enum Error {
    #[error("creation time is not available for the filesystem")]
    CreationTimeUnavailableForFilesystem,
    #[error("creation time is not available on this platform currently")]
    CreationTimeUnavailableForPlatform,
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.dir.root.join(OsStr::from_bytes(self.name_bytes()))
    }

    pub fn file_name(&self) -> OsString {
        OsStr::from_bytes(self.name_bytes()).to_os_string()
    }

    #[cfg(any(target_os = "linux", target_os = "emscripten", target_os = "android"))]
    pub fn metadata(&self) -> io::Result<FileAttr> {
        let fd = self.dir.fd;
        let name = self.entry.d_name.as_ptr();

        cfg_has_statx! {
            if let Some(ret) = unsafe { try_statx(
                fd,
                name,
                libc::AT_SYMLINK_NOFOLLOW | libc::AT_STATX_SYNC_AS_STAT,
                libc::STATX_ALL,
            ) } {
                return ret;
            }
        }

        let mut stat: stat64 = unsafe { mem::zeroed() };
        cvt(unsafe { fstatat64(fd, name, &mut stat, libc::AT_SYMLINK_NOFOLLOW) })?;
        Ok(FileAttr::from_stat64(stat))
    }

    #[cfg(not(any(target_os = "linux", target_os = "emscripten", target_os = "android")))]
    pub fn metadata(&self) -> io::Result<FileAttr> {
        lstat(&self.path())
    }

    #[cfg(not(any(
        target_os = "solaris",
        target_os = "illumos",
        target_os = "haiku",
        target_os = "vxworks"
    )))]
    pub fn file_type(&self) -> io::Result<FileType> {
        match self.entry.d_type {
            libc::DT_CHR => Ok(FileType {
                mode: libc::S_IFCHR,
            }),
            libc::DT_FIFO => Ok(FileType {
                mode: libc::S_IFIFO,
            }),
            libc::DT_LNK => Ok(FileType {
                mode: libc::S_IFLNK,
            }),
            libc::DT_REG => Ok(FileType {
                mode: libc::S_IFREG,
            }),
            libc::DT_SOCK => Ok(FileType {
                mode: libc::S_IFSOCK,
            }),
            libc::DT_DIR => Ok(FileType {
                mode: libc::S_IFDIR,
            }),
            libc::DT_BLK => Ok(FileType {
                mode: libc::S_IFBLK,
            }),
            _ => lstat(&self.path()).map(|m| m.file_type()),
        }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "linux",
        target_os = "emscripten",
        target_os = "android",
        target_os = "solaris",
        target_os = "illumos",
        target_os = "haiku",
        target_os = "l4re",
        target_os = "fuchsia",
        target_os = "redox",
        target_os = "vxworks"
    ))]
    pub fn ino(&self) -> u64 {
        self.entry.d_ino as u64
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "dragonfly"
    ))]
    fn name_bytes(&self) -> &[u8] {
        use crate::slice;
        unsafe {
            slice::from_raw_parts(
                self.entry.d_name.as_ptr() as *const u8,
                self.entry.d_namlen as usize,
            )
        }
    }
    #[cfg(any(
        target_os = "android",
        target_os = "linux",
        target_os = "emscripten",
        target_os = "l4re",
        target_os = "haiku",
        target_os = "vxworks"
    ))]
    fn name_bytes(&self) -> &[u8] {
        unsafe { CStr::from_ptr(self.entry.d_name.as_ptr()).to_bytes() }
    }

    pub fn file_name_os_str(&self) -> &OsStr {
        OsStr::from_bytes(self.name_bytes())
    }
}

fn cstr(path: &Path) -> io::Result<CString> {
    Ok(CString::new(path.as_os_str().as_bytes())?)
}

fn lstat(p: &Path) -> io::Result<FileAttr> {
    let p = cstr(p)?;

    cfg_has_statx! {
        if let Some(ret) = unsafe { try_statx(
            libc::AT_FDCWD,
            p.as_ptr(),
            libc::AT_SYMLINK_NOFOLLOW | libc::AT_STATX_SYNC_AS_STAT,
            libc::STATX_ALL,
        ) } {
            return ret;
        }
    }

    let mut stat: stat64 = unsafe { mem::zeroed() };
    cvt(unsafe { lstat64(p.as_ptr(), &mut stat) })?;
    Ok(FileAttr::from_stat64(stat))
}
