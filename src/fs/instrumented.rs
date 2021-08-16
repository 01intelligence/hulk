//! Instrument metrics for used filesystem operations from [`tokio::fs`].

use tokio::fs;

use crate::feature;
use crate::prelude::*;
use crate::utils::{DateTime, Path, PathBuf};

#[derive(Clone, Debug)]
pub struct OpenOptions(fs::OpenOptions);

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions(fs::OpenOptions::new())
    }

    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.0.read(read);
        self
    }

    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.0.write(write);
        self
    }

    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.0.append(append);
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.0.truncate(truncate);
        self
    }

    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.0.create(create);
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.0.create_new(create_new);
        self
    }

    pub async fn open(&self, path: impl AsRef<Path>) -> std::io::Result<fs::File> {
        // TODO: update metrics
        self.0.open(path.as_ref().as_std_path()).await
    }
}

feature! {
    #![unix]

    use std::os::unix::fs::OpenOptionsExt;

    impl OpenOptions {
       pub fn mode(&mut self, mode: u32) -> &mut OpenOptions {
            self.0.mode(mode);
            self
        }

        pub fn custom_flags(&mut self, flags: i32) -> &mut OpenOptions {
            self.0.custom_flags(flags);
            self
        }
    }
}

feature! {
    #![windows]

    use std::os::windows::fs::OpenOptionsExt;

    impl OpenOptions {
        pub fn access_mode(&mut self, access: u32) -> &mut OpenOptions {
            self.0.access_mode(access);
            self
        }

        pub fn share_mode(&mut self, share: u32) -> &mut OpenOptions {
            self.0.share_mode(share);
            self
        }

        pub fn custom_flags(&mut self, flags: u32) -> &mut OpenOptions {
            self.0.custom_flags(flags);
            self
        }

        pub fn attributes(&mut self, attributes: u32) -> &mut OpenOptions {
            self.0.attributes(attributes);
            self
        }

        pub fn security_qos_flags(&mut self, flags: u32) -> &mut OpenOptions {
            self.0.security_qos_flags(flags);
            self
        }
    }
}

impl From<fs::OpenOptions> for OpenOptions {
    fn from(options: fs::OpenOptions) -> OpenOptions {
        OpenOptions(options)
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn mkdir_all(path: impl AsRef<Path>, mode: u32) -> std::io::Result<()> {
    let mut b = fs::DirBuilder::new();
    // TODO: windows
    #[cfg(unix)]
    b.mode(mode);
    b.recursive(true).create(path.as_ref().as_std_path()).await
}

pub async fn rename(src_path: impl AsRef<Path>, dst_path: impl AsRef<Path>) -> std::io::Result<()> {
    fs::rename(src_path.as_ref(), dst_path.as_ref()).await
}

pub async fn remove(path: impl AsRef<Path>) -> std::io::Result<()> {
    let meta = fs::metadata(path.as_ref()).await?;
    if meta.is_dir() {
        fs::remove_dir(path.as_ref()).await
    } else {
        fs::remove_file(path.as_ref()).await
    }
}

pub async fn remove_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    let meta = fs::metadata(path.as_ref()).await?;
    if meta.is_dir() {
        fs::remove_dir_all(path.as_ref()).await
    } else {
        fs::remove_file(path.as_ref()).await
    }
}

pub async fn access(path: impl AsRef<Path>) -> std::io::Result<()> {
    use faccess::{AccessMode, PathExt};
    let path = path.as_ref().to_owned();
    super::asyncify(move || path.access(AccessMode::READ | AccessMode::WRITE)).await
}

pub async fn metadata(path: impl AsRef<Path>) -> std::io::Result<std::fs::Metadata> {
    fs::metadata(path.as_ref()).await
}

pub async fn canonicalize(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    fs::canonicalize(path.as_ref())
        .await
        .map(|p| p.try_into().unwrap())
}

pub trait MetadataExt {
    fn modified_at(&self) -> crate::utils::DateTime;
    fn accessed_at(&self) -> crate::utils::DateTime;
    fn created_at(&self) -> crate::utils::DateTime;
}

impl MetadataExt for std::fs::Metadata {
    fn modified_at(&self) -> DateTime {
        // Available on Unix/Windows, so unwrap is safe.
        self.modified().unwrap().into()
    }

    fn accessed_at(&self) -> DateTime {
        // Available on Unix/Windows, so unwrap is safe.
        self.accessed().unwrap().into()
    }

    fn created_at(&self) -> DateTime {
        // Available on Unix/Windows, so unwrap is safe.
        self.created().unwrap().into()
    }
}
