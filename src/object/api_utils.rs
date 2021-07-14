use const_format::concatcp;
use relative_path::{RelativePath, RelativePathBuf};
use tokio::io::AsyncRead;

use super::*;

pub const SLASH_SEPARATOR: &str = "/";

// System meta bucket.
pub const SYSTEM_META_BUCKET: &str = ".hulk.sys";
// Multipart meta prefix.
const MPART_META_PREFIX: &str = "multipart";
// System Multipart meta prefix.
const SYSTEM_META_MULTIPART_BUCKET: &str =
    concatcp!(SYSTEM_META_BUCKET, SLASH_SEPARATOR, MPART_META_PREFIX);
// System tmp meta prefix.
const SYSTEM_META_TMP_BUCKET: &str = concatcp!(SYSTEM_META_BUCKET, "/tmp");
// System tmp meta prefix for deleted objects.
const SYSTEM_META_TMP_DELETED_BUCKET: &str = concatcp!(SYSTEM_META_TMP_BUCKET, "/.trash");

// DNS separator (period), used for bucket name validation.
const DNS_DELIMITER: &str = ".";
// On compressed files bigger than this;
const COMP_READ_AHEAD_SIZE: usize = 100 << 20;
// Read this many buffers ahead.
const COMP_READ_AHEAD_BUFFERS: usize = 5;
// Size of each buffer.
const COMP_READ_AHEAD_BUF_SIZE: usize = 1 << 20;

// Join paths and retains trailing SlashSeparator of the last element.
pub fn path_join(elements: &[&str]) -> String {
    if elements.is_empty() {
        return "".to_owned();
    }
    let mut p = RelativePathBuf::new();
    for e in elements {
        p.push(e);
    }
    // Retail prefix slash.
    let mut s = if elements[0].starts_with(SLASH_SEPARATOR) {
        SLASH_SEPARATOR.to_owned() + &p.normalize().to_string()
    } else {
        p.normalize().to_string()
    };
    // Retail suffix slash.
    if elements[elements.len() - 1].ends_with(SLASH_SEPARATOR) {
        s.push_str(SLASH_SEPARATOR);
    }
    return s;
}

pub struct GetObjectReader {
    reader: Box<dyn AsyncRead>,
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
