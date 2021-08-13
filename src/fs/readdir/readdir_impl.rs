use std::ffi::{CStr, CString, OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, io, mem, ptr};

use lazy_static::lazy_static;
#[cfg(any(all(target_os = "linux", target_env = "gnu"), target_os = "macos",))]
use libc::c_char;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use libc::SYS_getdents as SYS_getdents64;
use libc::{c_int, mode_t};
/*
pub struct dirent64 {
    pub d_ino: ino64_t,
    pub d_off: off64_t,
    pub d_reclen: c_ushort,
    pub d_type: c_uchar,
    pub d_name: [c_char; 256],
}
*/
#[cfg(not(target_os = "linux"))]
use libc::{dirent as dirent64, lstat as lstat64, off_t as off64_t, stat as stat64};
#[cfg(any(target_os = "linux"))]
use libc::{dirent64, fstatat64, lstat64, off64_t, stat64, SYS_getdents64};
use memoffset::offset_of;
use thiserror::Error;

use crate::pool::{BytesPool, BytesPoolGuard};
use crate::sys::time::SystemTime;
#[cfg(any(all(target_os = "linux", target_env = "gnu"), target_os = "macos",))]
use crate::sys::weak::syscall;
use crate::sys::{cvt, cvt_r};

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

struct DirInfo {
    guard: BytesPoolGuard<'static, DIRENT_BUF_SIZE>, // guard of buffer for directory I/O
    nbuf: usize,                                     // length of buf; return value from getdents
    bufp: usize,                                     // location of next record in buf
}

pub struct ReadDir {
    inner: Arc<InnerReadDir>,
    dir_info: Option<DirInfo>,
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
    #[error("dirent buffer pool out of memory")]
    DirentBufferPoolOutOfMemory,
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
        unsafe {
            std::slice::from_raw_parts(
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

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DirEntry").field(&self.path()).finish()
    }
}

pub fn readdir(p: impl AsRef<Path>) -> io::Result<ReadDir> {
    let p = p.as_ref();
    let root = p.to_path_buf();
    let p = cstr(p)?;
    unsafe {
        let fd = libc::open(p.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY);
        if fd == -1 {
            Err(io::Error::last_os_error())
        } else {
            let inner = InnerReadDir { fd, root };
            Ok(ReadDir {
                inner: Arc::new(inner),
                dir_info: None,
            })
        }
    }
}

impl Drop for InnerReadDir {
    fn drop(&mut self) {
        let r = unsafe { libc::close(self.fd) };
        debug_assert_eq!(r, 0);
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        // check if any class (owner, group, others) has write permission
        self.mode & 0o222 == 0
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            // remove write permission for all classes; equivalent to `chmod a-w <file>`
            self.mode &= !0o222;
        } else {
            // add write permission for all classes; equivalent to `chmod a+w <file>`
            self.mode |= 0o222;
        }
    }
    pub fn mode(&self) -> u32 {
        self.mode as u32
    }
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.is(libc::S_IFDIR)
    }
    pub fn is_file(&self) -> bool {
        self.is(libc::S_IFREG)
    }
    pub fn is_symlink(&self) -> bool {
        self.is(libc::S_IFLNK)
    }

    pub fn is(&self, mode: mode_t) -> bool {
        self.mode & libc::S_IFMT == mode
    }
}

const BLOCK_SIZE: usize = 8192;
const DIRENT_BUF_SIZE: usize = BLOCK_SIZE * 128;
const DIRENT_BUF_POOL_MAX_SIZE: usize = 1024 * 8;

lazy_static! {
    static ref DIRENT_BUF_POOL: Arc<BytesPool<DIRENT_BUF_SIZE>> =
        Arc::new(BytesPool::new(DIRENT_BUF_POOL_MAX_SIZE));
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.inner.root, f)
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        // If no dir_info, create one.
        let d = if self.dir_info.is_none() {
            let guard = match tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async { DIRENT_BUF_POOL.get().await })
            }) {
                Err(err) => {
                    return Some(Err(io::Error::new(
                        io::ErrorKind::OutOfMemory,
                        Error::DirentBufferPoolOutOfMemory,
                    )));
                }
                Ok(guard) => guard,
            };

            self.dir_info.insert(DirInfo {
                guard,
                nbuf: 0,
                bufp: 0,
            })
        } else {
            self.dir_info.as_mut().unwrap()
        };

        let dirent_buf: &mut [u8] = &mut d.guard;
        loop {
            // Refill the buffer if necessary
            if d.bufp >= d.nbuf {
                d.bufp = 0;
                let fd = self.inner.fd;
                let buf_ptr = dirent_buf.as_mut_ptr() as *mut u8;
                let buf_len = dirent_buf.len();
                match cvt_r(|| unsafe { libc::syscall(SYS_getdents64, fd, buf_ptr, buf_len) }) {
                    Ok(n) => d.nbuf = n as usize,
                    Err(err) => return Some(Err(err)),
                }
                if d.nbuf <= 0 {
                    return None; // EOF
                }
            }

            // Drain the buffer
            let buf = &dirent_buf[d.bufp..d.nbuf];
            let entry: &dirent64 = unsafe { (buf.as_ptr() as *const dirent64).as_ref().unwrap() };
            if offset_of!(dirent64, d_reclen) + mem::size_of::<libc::c_ushort>() > buf.len() {
                return None; // EOF, ignore any unexpected condition
            }
            let rec_len = entry.d_reclen as usize;
            if rec_len > buf.len() {
                return None; // EOF, ignore any unexpected condition
            }
            let rec = &buf[..rec_len];
            d.bufp += rec_len; // advance location to next record
            let ino = entry.d_ino;
            if ino == 0 {
                continue;
            }
            let name_offset = offset_of!(dirent64, d_name);
            if name_offset >= rec_len {
                return None; // EOF, ignore any unexpected condition
            }
            let name_len = rec_len - name_offset;
            let mut name: &[u8] = &rec[name_offset..name_offset + name_len];
            for (i, c) in name.iter().enumerate() {
                if *c == 0 {
                    name = &name[..i]; // truncate tail NULL char
                    break;
                }
            }
            // Check for useless names before allocating a string.
            if name == b"." || name == b".." {
                continue;
            }
            return Some(Ok(DirEntry {
                entry: *entry,
                dir: Arc::clone(&self.inner),
            }));
        }
    }
}

fn cstr(path: &Path) -> io::Result<CString> {
    Ok(CString::new(path.as_os_str().as_bytes())?)
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    let p = cstr(p)?;

    cfg_has_statx! {
        if let Some(ret) = unsafe { try_statx(
            libc::AT_FDCWD,
            p.as_ptr(),
            libc::AT_STATX_SYNC_AS_STAT,
            libc::STATX_ALL,
        ) } {
            return ret;
        }
    }

    let mut stat: stat64 = unsafe { mem::zeroed() };
    cvt(unsafe { stat64(p.as_ptr(), &mut stat) })?;
    Ok(FileAttr::from_stat64(stat))
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
