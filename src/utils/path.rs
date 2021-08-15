//! Export Path/PathBuf which only contain UTF-8 encoded bytes.
//! Refer: https://github.com/withoutboats/camino

use std::io;

use crate::prelude::*;

pub type Path = camino::Utf8Path;
pub type PathBuf = camino::Utf8PathBuf;
pub type Component<'a> = camino::Utf8Component<'a>;
pub type Prefix<'a> = camino::Utf8Prefix<'a>;

pub trait PathExt {
    fn as_str(&self) -> &str;
}

impl PathExt for std::path::Path {
    fn as_str(&self) -> &str {
        self.to_str().expect("path contains non-UTF-8 bytes")
    }
}

pub trait PathAbsolutize {
    fn absolutize(&self) -> io::Result<Cow<Path>>;
}

impl PathAbsolutize for Path {
    fn absolutize(&self) -> io::Result<Cow<Path>> {
        use path_absolutize::Absolutize;
        let path: Cow<Path> = match Absolutize::absolutize(self.as_std_path())? {
            Cow::Borrowed(path) => Cow::Borrowed(
                path.try_into()
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
            ),
            Cow::Owned(path) => Cow::Owned(
                path.try_into()
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
            ),
        };
        Ok(path)
    }
}
