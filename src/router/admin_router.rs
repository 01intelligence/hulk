use const_format::concatcp;

use crate::globals;

pub const ADMIN_PATH_PREFIX: &str = concatcp!(globals::SYSTEM_RESERVED_BUCKET_PATH, "/admin");
pub const ADMIN_API_VERSION: &str = "v3";
pub const ADMIN_API_VERSION_PREFIX: &str = concatcp!(globals::SLASH_SEPARATOR, ADMIN_API_VERSION);
