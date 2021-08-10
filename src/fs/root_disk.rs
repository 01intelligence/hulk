#[cfg(target_family = "unix")]
fn same_disk(disk1: &str, disk2: &str) -> anyhow::Result<bool> {
    use nix::sys::stat::lstat;
    let st1 = lstat(disk1)?;
    let st2 = lstat(disk2)?;
    Ok(st1.st_dev == st2.st_dev)
}

#[cfg(target_family = "windows")]
fn same_disk(disk1: &str, disk2: &str) -> anyhow::Result<bool> {
    Ok(false)
}

pub fn is_root_disk(disk_path: &str, root_disk: &str) -> anyhow::Result<bool> {
    same_disk(disk_path, root_disk)
}
