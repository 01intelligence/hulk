mod root_disk;
use std::collections::HashMap;
use std::path::Path;

use lazy_static::lazy_static;
use maplit::hashmap;
pub use root_disk::*;

pub struct Info {
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub files: u64,
    pub ffree: u64,
    pub fs_type: String,
}

#[cfg(target_family = "windows")]
pub async fn get_info<P: AsRef<Path>>(path: P) -> anyhow::Result<Info> {
    Ok(Info {
        total: 0,
        free: 0,
        used: 0,
        files: 0,
        ffree: 0,
        fs_type: "".to_string(),
    })
}

#[cfg(target_family = "unix")]
pub async fn get_info<P: AsRef<Path>>(path: P) -> anyhow::Result<Info> {
    use nix::sys::statfs::{statfs, Statfs};
    use nix::NixPath;
    let s = statfs(path.as_ref())?;

    // https://stackoverflow.com/questions/54823541/what-do-f-bsize-and-f-frsize-in-struct-statvfs-stand-for

    let info = Info {
        total: s.block_size() as u64
            * (s.blocks() - (s.blocks_free() - s.blocks_available())) as u64,
        free: s.block_size() as u64 * s.blocks_available() as u64,
        used: s.block_size() as u64 * (s.blocks() - s.blocks_free()) as u64,
        files: s.files() as u64,
        ffree: s.files_free() as u64,
        fs_type: get_fs_type(s.filesystem_type()).to_owned(),
    };

    // Check for overflows.
    // XFS can show wrong values at times error out in such scenarios.
    if info.free > info.total {
        anyhow::bail!("detected free space ({}) > total disk space ({}), fs corruption at ({}). please run 'fsck'", info.free, info.total, path.as_ref().to_string_lossy());
    }

    Ok(info)
}

#[cfg(target_family = "unix")]
fn get_fs_type(fs_type: nix::sys::statfs::FsType) -> &'static str {
    FS_TYPE_STRING_MAP
        .get(&(fs_type.0 as u64))
        .map(|t| *t)
        .unwrap_or_else(|| "UNKNOWN")
}

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
