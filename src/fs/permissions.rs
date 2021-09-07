use std::fs::Permissions as StdPermissions;

use crate::feature;
use crate::utils::Path;

#[derive(Clone, Debug)]
pub struct Permissions(StdPermissions);

impl Permissions {
    pub fn readonly(&self) -> bool {
        self.0.readonly()
    }

    pub async fn set_readonly(&mut self, readonly: bool) {
        self.0.set_readonly(readonly)
    }
}

feature! {
    #![unix]

    pub use std::os::unix::fs::PermissionsExt;

    impl PermissionsExt for Permissions {
        fn mode(&self) -> u32 {
            self.0.mode()
        }

        fn set_mode(&mut self, mode: u32) {
            self.0.set_mode(mode);
        }

        fn from_mode(mode: u32) -> Permissions {
            Permissions(StdPermissions::from_mode(mode))
        }
    }
}

pub async fn set_permissions(path: impl AsRef<Path>, perm: Permissions) -> std::io::Result<()> {
    tokio::fs::set_permissions(path.as_ref().as_std_path(), perm.0).await
}
