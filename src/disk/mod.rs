mod root_disk;

use std::path::Path;

pub use root_disk::*;

pub struct Info {
    pub total: u64,
    pub free: u64,
    pub used: u64,
    pub files: u64,
    pub ffree: u64,
    pub fs_type: String,
}

pub async fn get_info<P: AsRef<Path>>(path: P) -> anyhow::Result<Info> {
    let disk_usage = heim::disk::usage(path).await?;

    Ok(Info {
        total: disk_usage.total().get::<heim::units::information::byte>(),
        free: disk_usage.free().get::<heim::units::information::byte>(),
        used: disk_usage.used().get::<heim::units::information::byte>(),
        files: 0,               // todo
        ffree: 0,               // todo
        fs_type: "".to_owned(), // todo
    })
}
