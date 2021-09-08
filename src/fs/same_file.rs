use super::*;

pub trait SameFile {
    fn is_same_file(&self, other: &Self) -> bool;
}

#[cfg(unix)]
impl SameFile for std::fs::Metadata {
    fn is_same_file(&self, other: &Self) -> bool {
        use std::os::unix::fs::MetadataExt;
        self.dev() == other.dev()
            && self.ino() == other.ino()
            && self.modified().unwrap() == other.modified().unwrap() // always available for Unix
            && self.mode() == other.mode()
            && self.len() == other.len()
    }
}

#[cfg(windows)]
impl SameFile for std::fs::Metadata {
    fn is_same_file(&self, other: &Self) -> bool {
        use std::os::windows::fs::MetadataExt;
        self.volume_serial_number() == other.volume_serial_number()
            && self.file_index() == other.file_index()
            && self.modified().unwrap() == other.modified().unwrap() // always available for Windows
            && self.file_attributes() == other.file_attributes() // TODO
            && self.len() == other.len()
    }
}
