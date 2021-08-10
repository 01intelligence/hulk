use std::path::Path;

pub fn access(path: impl AsRef<Path>) -> std::io::Result<()> {
    use faccess::{AccessMode, PathExt};
    path.as_ref().access(AccessMode::READ | AccessMode::WRITE)
}
