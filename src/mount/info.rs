use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::ensure;

#[cfg(target_os = "linux")]
pub fn check_cross_device<P: AsRef<Path>>(abs_paths: &[P]) -> anyhow::Result<()> {
    use procfs::process::Process;

    let process = Process::myself()?;
    let mounts = process.mountinfo()?;

    for path in abs_paths {
        let path = path.as_ref();
        ensure!(
            path.is_absolute(),
            "invalid argument, path '{}' is expected to be absolute",
            path.to_str().unwrap()
        );
        let mut cross_mounts = Vec::new();
        for mount in &mounts {
            if mount.mount_point.starts_with(path) && mount.mount_point != path {
                cross_mounts.push(&mount.mount_point);
            }
        }
        ensure!(cross_mounts.is_empty(), "cross-device mounts detected on path '{}' at following locations {:?}. Export path should not have any sub-mounts, refusing to start", path.to_str().unwrap(), cross_mounts);
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn check_cross_device<P: AsRef<Path>>(abs_paths: &[P]) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn is_likely_mount_point<P: AsRef<Path>>(path: P) -> bool {
    use nix::sys::stat::{lstat, SFlag};
    let s1 = match lstat(path.as_ref()) {
        Err(_) => {
            return false;
        }
        Ok(s1) => s1,
    };

    // A symlink can never be a mount point
    if SFlag::from_bits_truncate(s1.st_mode).contains(SFlag::S_IFLNK) {
        return false;
    }

    let s2 = match lstat(path.as_ref().parent().unwrap_or_else(|| Path::new("/"))) {
        Err(_) => {
            return false;
        }
        Ok(s2) => s2,
    };

    // If the directory has a different device as parent, then it is a mountpoint.
    if s1.st_dev != s2.st_dev {
        return true;
    }

    // The same i-node as path - this check is for bind mounts.
    return s1.st_ino == s2.st_ino;
}

#[cfg(target_os = "windows")]
pub fn is_likely_mount_point<P: AsRef<Path>>(path: P) -> bool {
    use std::os::windows::prelude::*;

    use winbinding::Windows::Win32::Foundation::PWSTR;
    use winbinding::Windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW};
    use winbinding::Windows::Win32::System::WindowsProgramming::*;
    let mut path: Vec<u16> = path.as_ref().as_os_str().encode_wide().collect();
    let mut volume = vec![0u16; path.len()];
    let success = unsafe {
        GetVolumePathNameW(
            PWSTR(&mut path[0] as *mut u16),
            PWSTR(&mut volume[0] as *mut u16),
            volume.len() as u32,
        )
        .as_bool()
    };
    if !success {
        return false;
    }
    let drive_type = unsafe { GetDriveTypeW(PWSTR(&mut volume[0] as *mut u16)) };
    match drive_type {
        // Recognize "fixed", "removable", "remote" and "ramdisk" drives as proper drives
        // which can be treated as an actual mount-point, rest can be ignored.
        // https://docs.microsoft.com/en-us/windows/desktop/api/fileapi/nf-fileapi-getdrivetypew
        DRIVE_FIXED | DRIVE_REMOVABLE | DRIVE_REMOTE | DRIVE_RAMDISK => true,
        _ => false,
    }
}

#[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
pub fn is_likely_mount_point<P: AsRef<Path>>(path: P) -> bool {
    false
}
