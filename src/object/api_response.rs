use crate::metacache;

pub const MAX_OBJECT_LIST: usize =
    metacache::METACACHE_BLOCK_SIZE - (metacache::METACACHE_BLOCK_SIZE / 10);
pub const MAX_DELETE_LIST: usize = 10000;
pub const MAX_UPLOAD_LIST: usize = 10000;
pub const MAX_PARTS_LIST: usize = 10000;
