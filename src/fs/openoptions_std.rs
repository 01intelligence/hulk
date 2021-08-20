use std::fs;

use crate::feature;
use crate::utils::Path;

#[derive(Clone, Debug)]
pub struct StdOpenOptions(
    fs::OpenOptions,
    #[cfg(unix)] pub(super) i32,
    #[cfg(windows)] pub(super) u32,
);

impl StdOpenOptions {
    pub fn new() -> Self {
        StdOpenOptions(fs::OpenOptions::new(), 0)
    }

    pub fn read(&mut self, read: bool) -> &mut Self {
        self.0.read(read);
        self
    }

    pub fn write(&mut self, write: bool) -> &mut Self {
        self.0.write(write);
        self
    }

    pub fn append(&mut self, append: bool) -> &mut Self {
        self.0.append(append);
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.0.truncate(truncate);
        self
    }

    pub fn create(&mut self, create: bool) -> &mut Self {
        self.0.create(create);
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.0.create_new(create_new);
        self
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> std::io::Result<fs::File> {
        // TODO: update metrics
        self.0.open(path.as_ref().as_std_path())
    }
}

feature! {
    #![unix]

    use std::os::unix::fs::OpenOptionsExt;

    impl StdOpenOptions {
       pub fn mode(&mut self, mode: u32) -> &mut StdOpenOptions {
            self.0.mode(mode);
            self
        }

        pub fn custom_flags(&mut self, flags: i32) -> &mut StdOpenOptions {
            self.1 = flags; // cache for subsequent appending
            self.0.custom_flags(flags);
            self
        }

        pub fn append_custom_flags(&mut self, flags: i32) -> &mut StdOpenOptions {
            self.1 |= flags; // cache for subsequent appending
            self.0.custom_flags(self.1);
            self
        }
    }
}

feature! {
    #![windows]

    use std::os::windows::fs::OpenOptionsExt;

    impl StdOpenOptions {
        pub fn access_mode(&mut self, access: u32) -> &mut StdOpenOptions {
            self.0.access_mode(access);
            self
        }

        pub fn share_mode(&mut self, share: u32) -> &mut StdOpenOptions {
            self.0.share_mode(share);
            self
        }

        pub fn custom_flags(&mut self, flags: u32) -> &mut StdOpenOptions {
            self.1 = flags; // cache for subsequent appending
            self.0.custom_flags(flags);
            self
        }

        pub fn append_custom_flags(&mut self, flags: u32) -> &mut StdOpenOptions {
            self.1 |= flags; // cache for subsequent appending
            self.0.custom_flags(self.1);
            self
        }

        pub fn attributes(&mut self, attributes: u32) -> &mut StdOpenOptions {
            self.0.attributes(attributes);
            self
        }

        pub fn security_qos_flags(&mut self, flags: u32) -> &mut StdOpenOptions {
            self.0.security_qos_flags(flags);
            self
        }
    }
}

impl Default for StdOpenOptions {
    fn default() -> Self {
        Self::new()
    }
}
