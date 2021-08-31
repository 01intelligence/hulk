use std::collections::HashMap;

use lazy_static::lazy_static;
use maplit::hashmap;

use crate::utils::Path;

pub async fn get_disk_info(disk_path: &str) -> anyhow::Result<Info> {
    super::check_path_length(disk_path)?;
    match get_info(disk_path).await {
        Ok(info) => Ok(info),
        Err(err) => {
            // TODO: error wrap
            Err(err)
        }
    }
}

pub struct Info {
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub files: u64,
    pub ffree: u64,
    pub fs_type: String,
}

#[cfg(target_family = "unix")]
pub async fn get_info<P: AsRef<Path>>(path: P) -> anyhow::Result<Info> {
    use nix::sys::statfs::{statfs, Statfs};
    use nix::NixPath;
    let s = statfs(path.as_ref().as_std_path())?;

    // https://stackoverflow.com/questions/54823541/what-do-f-bsize-and-f-frsize-in-struct-statvfs-stand-for

    let info = Info {
        total: s.block_size() as u64
            * (s.blocks() - (s.blocks_free() - s.blocks_available())) as u64,
        free: s.block_size() as u64 * s.blocks_available() as u64,
        used: s.block_size() as u64 * (s.blocks() - s.blocks_free()) as u64,
        files: s.files() as u64,
        ffree: s.files_free() as u64,
        fs_type: get_fs_type(&s).to_owned(),
    };

    // Check for overflows.
    // XFS can show wrong values at times error out in such scenarios.
    if info.free > info.total {
        anyhow::bail!("detected free space ({}) > total disk space ({}), fs corruption at ({}). please run 'fsck'", info.free, info.total, path.as_ref().to_string_lossy());
    }

    Ok(info)
}

#[cfg(target_family = "windows")]
pub async fn get_info<P: AsRef<Path>>(path: P) -> anyhow::Result<Info> {
    use winbinding::Windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDiskFreeSpaceW};

    let mut free_bytes_available = 0u64;
    let mut total_number_of_bytes = 0u64;
    let mut total_number_of_free_bytes = 0u64;
    unsafe {
        // https://microsoft.github.io/windows-docs-rs/doc/bindings/Windows/Win32/Storage/FileSystem/fn.GetDiskFreeSpaceExW.html
        let _ = GetDiskFreeSpaceExW(
            path.as_ref().to_str().unwrap(),
            &mut free_bytes_available as *mut u64,
            &mut total_number_of_bytes as *mut u64,
            &mut total_number_of_free_bytes as *mut u64,
        )
        .ok()?;
    }

    let mut sectors_per_cluster = 0u32;
    let mut bytes_per_sector = 0u32;
    let mut number_of_free_clusters = 0u32;
    let mut total_number_of_clusters = 0u32;
    unsafe {
        // https://microsoft.github.io/windows-docs-rs/doc/bindings/Windows/Win32/Storage/FileSystem/fn.GetDiskFreeSpaceW.html
        let _ = GetDiskFreeSpaceW(
            path.as_ref().to_str().unwrap(),
            &mut sectors_per_cluster as *mut u32,
            &mut bytes_per_sector as *mut u32,
            &mut number_of_free_clusters as *mut u32,
            &mut total_number_of_clusters as *mut u32,
        )
        .ok()?;
    }

    let info = Info {
        total: total_number_of_bytes,
        free: total_number_of_free_bytes,
        used: total_number_of_bytes - total_number_of_free_bytes,
        files: total_number_of_clusters as u64,
        ffree: number_of_free_clusters as u64,
        fs_type: get_fs_type(path.as_ref()),
    };

    if info.free > info.total {
        anyhow::bail!("detected free space ({}) > total disk space ({}), fs corruption at ({}). please run 'fsck'", info.free, info.total, path.as_ref().to_string_lossy());
    }

    Ok(info)
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "android"))]
fn get_fs_type(stat: &nix::sys::statfs::Statfs) -> &'static str {
    FS_TYPE_STRING_MAP
        .get(&(stat.filesystem_type().0 as u64))
        .map(|t| *t)
        .unwrap_or_else(|| "UNKNOWN")
}

#[cfg(all(
    target_family = "unix",
    not(any(target_os = "linux", target_os = "android"))
))]
fn get_fs_type(stat: &nix::sys::statfs::Statfs) -> &str {
    stat.filesystem_type_name()
}

#[cfg(target_family = "windows")]
fn get_fs_type(path: &Path) -> String {
    use std::ffi::{OsStr, OsString};

    use crate::utils::{Component, Prefix};

    let path = match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(_) | Prefix::UNC(_, _) => prefix.as_os_str(),
            _ => OsStr::new(""),
        },
        _ => OsStr::new(""),
    };
    use std::os::windows::prelude::*;
    let mut path: Vec<u16> = path.encode_wide().collect();

    use winbinding::Windows::Win32::Foundation::PWSTR;
    use winbinding::Windows::Win32::Storage::FileSystem::GetVolumeInformationW;

    let volume_name_size = 260u32;
    let file_system_name_size = 260u32;
    let mut volume_serial_number = 0u32;
    let mut file_system_flags = 0u32;
    let mut maximum_component_length = 0u32;
    let mut file_system_name_buffer = [0u16; 260];
    let mut volume_name = [0u16; 260];

    let success = unsafe {
        // https://microsoft.github.io/windows-docs-rs/doc/bindings/Windows/Win32/Foundation/struct.PWSTR.html
        GetVolumeInformationW(
            PWSTR(&mut path[0] as *mut u16),
            PWSTR(&mut volume_name[0] as *mut u16),
            volume_name_size,
            &mut volume_serial_number as *mut u32,
            &mut maximum_component_length as *mut u32,
            &mut file_system_flags as *mut u32,
            PWSTR(&mut file_system_name_buffer[0] as *mut u16),
            file_system_name_size,
        )
        .as_bool()
    };
    if !success {
        return "".to_owned();
    }

    OsString::from_wide(&file_system_name_buffer[..])
        .into_string()
        .unwrap_or_default()
}

#[cfg(target_family = "unix")]
lazy_static! {
    static ref FS_TYPE_STRING_MAP: HashMap<u64, &'static str> = hashmap! {
        0x1021994 => "TMPFS",
        0x137d => "EXT",
        0x4244 => "HFS",
        0x4d44 => "MSDOS",
        0x52654973 => "REISERFS",
        0x5346544e => "NTFS",
        0x58465342 => "XFS",
        0x61756673 => "AUFS",
        0x6969 => "NFS",
        0xef51 => "EXT2OLD",
        0xef53 => "EXT4",
        0xf15f => "ecryptfs",
        0x794c7630 => "overlayfs",
        0x2fc12fc1 => "zfs",
        0xff534d42 => "cifs",
        0x53464846 => "wslfs",
    };
}
