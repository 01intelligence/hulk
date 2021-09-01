use const_format::concatcp;
use relative_path::{RelativePath, RelativePathBuf};
use tokio::io::AsyncRead;

use super::*;
use crate::prelude::*;
use crate::utils::Path;

pub const SLASH_SEPARATOR: &str = "/";

// System meta bucket.
pub const SYSTEM_META_BUCKET: &str = ".hulk.sys";
// Multipart meta prefix.
pub const MPART_META_PREFIX: &str = "multipart";
// System Multipart meta prefix.
pub const SYSTEM_META_MULTIPART_BUCKET: &str =
    concatcp!(SYSTEM_META_BUCKET, SLASH_SEPARATOR, MPART_META_PREFIX);
// System tmp meta prefix.
pub const SYSTEM_META_TMP_BUCKET: &str = concatcp!(SYSTEM_META_BUCKET, "/tmp");
// System tmp meta prefix for deleted objects.
pub const SYSTEM_META_TMP_DELETED_BUCKET: &str = concatcp!(SYSTEM_META_TMP_BUCKET, "/.trash");

// DNS separator (period), used for bucket name validation.
const DNS_DELIMITER: &str = ".";
// On compressed files bigger than this;
const COMP_READ_AHEAD_SIZE: usize = 100 << 20;
// Read this many buffers ahead.
const COMP_READ_AHEAD_BUFFERS: usize = 5;
// Size of each buffer.
const COMP_READ_AHEAD_BUF_SIZE: usize = 1 << 20;

/// Tests whether the path refers to a directory.
pub fn path_is_dir(path: &str) -> bool {
    path.ends_with(SLASH_SEPARATOR)
}

/// Retain trailing slash to ensure directory.
pub fn path_ensure_dir(path: &str) -> Cow<str> {
    if path.is_empty() || path.ends_with(SLASH_SEPARATOR) {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(path.to_owned() + SLASH_SEPARATOR)
    }
}

/// Join paths and trim tailing slash.
pub fn path_join_not_dir(elements: &[&str]) -> String {
    path_join_inner(elements, false)
}

/// Join paths and retains trailing slash of the last element.
pub fn path_join(elements: &[&str]) -> String {
    path_join_inner(elements, true)
}

fn path_join_inner(elements: &[&str], retain_dir: bool) -> String {
    if elements.is_empty() {
        return "".to_owned();
    }
    let mut p = RelativePathBuf::new();
    for e in elements {
        p.push(e);
    }
    let mut s = if elements[0].starts_with(SLASH_SEPARATOR) {
        // Retail prefix slash.
        SLASH_SEPARATOR.to_owned() + &p.normalize().to_string()
    } else {
        p.normalize().to_string()
    };
    if retain_dir && elements[elements.len() - 1].ends_with(SLASH_SEPARATOR) {
        // Retail suffix slash.
        s.push_str(SLASH_SEPARATOR);
    }
    return s;
}

pub struct GetObjectReader {
    pub reader: Box<dyn AsyncRead + Unpin>,
    pub obj_info: ObjectInfo,
    cleanup_fns: Vec<Box<dyn Fn()>>,
    opts: ObjectOptions,
}

pub struct PutObjectReader {}

pub fn compress_self_test() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_join() {
        let cases = vec![
            (vec![], ""),
            (vec![""], ""),
            (vec!["a"], "a"),
            (vec!["a", "b"], "a/b"),
            (vec!["a", ""], "a"),
            (vec!["", "b"], "b"),
            (vec!["/", "a"], "/a"),
            (vec!["/", ""], "/"),
            (vec!["a/", "b"], "a/b"),
            (vec!["a/", ""], "a"),
            (vec!["", ""], ""),
            (vec!["a", "b/"], "a/b/"),
        ];
        for (elements, path) in cases {
            assert_eq!(path_join(&elements), path);
        }
    }
}
