//! OsStr/OsString/Path/PathBuf extra methods.

use std::ffi::OsStr;
use std::io;
use std::path::Path;

pub trait OsStrExt {
    fn as_str(&self) -> io::Result<&str>;
}

impl OsStrExt for OsStr {
    fn as_str(&self) -> io::Result<&str> {
        self.to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "OsStr contains non-UTF-8 bytes"))
    }
}

impl OsStrExt for Path {
    fn as_str(&self) -> io::Result<&str> {
        self.to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "path contains non-UTF-8 bytes"))
    }
}
