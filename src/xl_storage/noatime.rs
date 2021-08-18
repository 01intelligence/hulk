use crate::fs::OpenOptions;

pub trait OpenOptionsNoAtime {
    /// Read while do not update access time.
    fn no_atime(&mut self) -> &mut OpenOptions;
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
impl OpenOptionsNoAtime for OpenOptions {
    fn no_atime(&mut self) -> &mut OpenOptions {
        self.append_custom_flags(libc::O_NOATIME)
    }
}

#[cfg(any(target_family = "windows", target_os = "macos"))]
impl OpenOptionsNoAtime for OpenOptions {
    fn no_atime(&mut self) -> &mut OpenOptions {
        // Nothing for Windows/macOS
        self
    }
}

pub trait OpenOptionsSync {
    // Write with no buffering.
    fn sync(&mut self) -> &mut OpenOptions;
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
impl OpenOptionsSync for OpenOptions {
    fn sync(&mut self) -> &mut OpenOptions {
        self.append_custom_flags(libc::O_DSYNC)
    }
}

#[cfg(any(target_family = "windows", target_os = "macos"))]
impl OpenOptionsSync for OpenOptions {
    fn sync(&mut self) -> &mut OpenOptions {
        self.append_custom_flags(0x01000)
    }
}
