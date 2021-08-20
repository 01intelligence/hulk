use tokio::fs;

use crate::feature;
use crate::utils::Path;

#[derive(Clone, Debug)]
pub struct OpenOptions(
    fs::OpenOptions,
    #[cfg(unix)] pub(super) i32,
    #[cfg(windows)] pub(super) u32,
);

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions(fs::OpenOptions::new(), 0)
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
            self.1 = flags; // cache for subsequent appending
            self.0.custom_flags(flags);
            self
        }

        pub fn append_custom_flags(&mut self, flags: i32) -> &mut OpenOptions {
            self.1 |= flags; // cache for subsequent appending
            self.0.custom_flags(self.1);
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
            self.1 = flags; // cache for subsequent appending
            self.0.custom_flags(flags);
            self
        }

        pub fn append_custom_flags(&mut self, flags: u32) -> &mut OpenOptions {
            self.1 |= flags; // cache for subsequent appending
            self.0.custom_flags(self.1);
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

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}
