use std::io;

use super::readdir_impl;
use super::time::SystemTime;
use crate::sys::{AsInner, FromInner};

#[derive(Clone)]
pub struct Metadata(pub(super) readdir_impl::FileAttr);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Permissions(readdir_impl::FilePermissions);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FileType(pub(super) readdir_impl::FileType);

impl Metadata {
    pub fn file_type(&self) -> FileType {
        FileType(self.0.file_type())
    }

    pub fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.file_type().is_file()
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }

    pub fn len(&self) -> u64 {
        self.0.size()
    }

    pub fn permissions(&self) -> Permissions {
        Permissions(self.0.perm())
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        self.0.modified().map(FromInner::from_inner)
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        self.0.accessed().map(FromInner::from_inner)
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        self.0.created().map(FromInner::from_inner)
    }
}

impl Permissions {
    pub fn readonly(&self) -> bool {
        self.0.readonly()
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        self.0.set_readonly(readonly)
    }
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.0.is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.0.is_file()
    }

    pub fn is_symlink(&self) -> bool {
        self.0.is_symlink()
    }
}

impl AsInner<readdir_impl::FileType> for FileType {
    fn as_inner(&self) -> &readdir_impl::FileType {
        &self.0
    }
}

impl FromInner<readdir_impl::FilePermissions> for Permissions {
    fn from_inner(f: readdir_impl::FilePermissions) -> Permissions {
        Permissions(f)
    }
}

impl AsInner<readdir_impl::FilePermissions> for Permissions {
    fn as_inner(&self) -> &readdir_impl::FilePermissions {
        &self.0
    }
}
