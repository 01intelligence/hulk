use super::*;

pub(super) fn get_xl_disk_loc(disk_id: &str) -> (isize, isize, isize) {
    // TODO
    (-1, -1, -1)
}

pub(super) fn is_null_version_id(version_id: &str) -> bool {
    version_id.is_empty() || version_id == super::NULL_VERSION_ID
}
