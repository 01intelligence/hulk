use crate::errors::StorageError;

pub fn check_path_length(path_name: &str) -> anyhow::Result<(), StorageError> {
    // Apple OS X path length is limited to 1016.
    if cfg!(macos) && path_name.len() > 1016 {
        return Err(StorageError::FileNameTooLong);
    }

    // Disallow more than 1024 characters on windows, there
    // are no known name_max limits on Windows.
    if cfg!(windows) && path_name.len() > 1024 {
        return Err(StorageError::FileNameTooLong);
    }

    // On Unix we reject paths if they are just '.', '..' or '/'.
    if path_name == "." || path_name == ".." || path_name == crate::globals::SLASH_SEPARATOR {
        return Err(StorageError::FileAccessDenied);
    }

    // Check each path segment length is > 255 on all Unix
    // platforms, look for this value as NAME_MAX in
    // /usr/include/linux/limits.h
    let mut count = 0;
    for p in path_name.chars() {
        match p {
            '/' => {
                count = 0;
            }
            '\\' => {
                if cfg!(windows) {
                    count = 0;
                }
            }
            _ => {
                count += 1;
                if count > 255 {
                    return Err(StorageError::FileNameTooLong);
                }
            }
        }
    }

    Ok(())
}
